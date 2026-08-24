using System.Diagnostics.Eventing.Reader;
using System.Text.RegularExpressions;

namespace PboStudio.Core;

public sealed partial record WheaEvent(
    int EventId, DateTime Time, bool IsWarning, string Description, int? ApicId = null)
{
    public override string ToString() =>
        $"[{Time:yyyy-MM-dd HH:mm:ss}] WHEA {EventId} ({(IsWarning ? "warning" : "error")}): {Description}";

    // Matches both the English "Processor APIC ID: 0" and localised variants such as the German
    // "Prozessor-APIC-ID: 0", by anchoring on APIC rather than on the surrounding words.
    [GeneratedRegex(@"APIC[-\s]?ID:?\s*(\d+)", RegexOptions.IgnoreCase)]
    private static partial Regex ApicPattern();

    internal static int? ParseApicId(string description)
    {
        var m = ApicPattern().Match(description);
        return m.Success && int.TryParse(m.Groups[1].Value, out int id) ? id : null;
    }
}

/// <summary>
/// Watches the Windows event log for machine check exceptions.
///
/// This is the detection that matters most for Curve Optimizer work. An undervolted core
/// usually does not crash outright — it produces a *corrected* machine check (event 19) and
/// the machine keeps running as if nothing happened. Without watching for these, a value
/// that is quietly corrupting work looks indistinguishable from a stable one.
/// </summary>
public sealed class WheaWatcher : IDisposable
{
    private const string Provider = "Microsoft-Windows-WHEA-Logger";

    private static readonly string Query =
        $"*[System[Provider[@Name='{Provider}']]]";

    private EventLogWatcher? _watcher;

    public event Action<WheaEvent>? Raised;

    /// <summary>Machine checks already in the log, newest first. Useful as a "before" baseline.</summary>
    public static IReadOnlyList<WheaEvent> ReadExisting(int max = 20)
    {
        var found = new List<WheaEvent>();
        try
        {
            var query = new EventLogQuery("System", PathType.LogName, Query) { ReverseDirection = true };
            using var reader = new EventLogReader(query);

            for (EventRecord? rec = reader.ReadEvent(); rec is not null && found.Count < max; rec = reader.ReadEvent())
            {
                using (rec) found.Add(Convert(rec));
            }
        }
        catch (EventLogException)
        {
            // No WHEA provider on this machine, or the log is not readable. Not fatal.
        }
        return found;
    }

    public void Start()
    {
        var query = new EventLogQuery("System", PathType.LogName, Query);
        _watcher = new EventLogWatcher(query);
        _watcher.EventRecordWritten += (_, e) =>
        {
            if (e.EventRecord is { } rec)
                using (rec) Raised?.Invoke(Convert(rec));
        };
        _watcher.Enabled = true;
    }

    private static WheaEvent Convert(EventRecord rec)
    {
        string description;
        try { description = rec.FormatDescription() ?? "(no description)"; }
        catch { description = "(description unavailable)"; }

        // Level 3 is Warning; anything more severe counts as a hard error.
        bool isWarning = rec.Level == 3;
        string text = description.ReplaceLineEndings(" ").Trim();

        return new WheaEvent(
            rec.Id,
            rec.TimeCreated ?? DateTime.Now,
            isWarning,
            text,
            WheaEvent.ParseApicId(text));
    }

    /// <summary>
    /// Maps the APIC id a machine check reports to a physical core index.
    ///
    /// On Zen the APIC ids line up with logical processor numbers, so the core owning that
    /// logical processor is the one that faulted. This is a convention rather than a guarantee -
    /// it holds on the parts this tool targets, and returns null instead of guessing if the id
    /// does not match anything known.
    /// </summary>
    public static int? CoreForApicId(int apicId, IReadOnlyList<PhysicalCore> cores)
    {
        foreach (var core in cores)
            if (core.LogicalProcessors.Contains(apicId)) return core.Index;

        return null;
    }

    public void Dispose()
    {
        if (_watcher is null) return;
        _watcher.Enabled = false;
        _watcher.Dispose();
        _watcher = null;
    }
}
