using System.Text.Json;

namespace PboStudio.Core;

/// <summary>A fatal machine check Windows logged, mapped back to a physical core.</summary>
public sealed record WheaCrash(int Core, DateTime Time, int EventId);

/// <summary>
/// Picks up the crashes that happened while PboStudio was not running.
///
/// The auto-tuner only ever learned from its own test slots, which is the smaller half of the
/// evidence. A Curve Optimizer value that survives hours of Prime95 can still take the machine
/// down during an ordinary game: a light load lets the core boost to its highest clock, where a
/// negative offset has the least voltage left, and the engine's continuous full load never goes
/// near that state. When that happens the only record is a WHEA entry in the Windows event log
/// — which nothing read.
///
/// Nothing here writes to the core memory on its own. What was actually applied at the moment
/// of a crash cannot be known for certain: values written live are lost in the reboot, so the
/// margin a core holds afterwards may be milder than the one that failed. Recording that
/// automatically would cap the core too tightly on a guess. The crashes are surfaced with the
/// values they are about, and the reader confirms.
/// </summary>
public static class WheaCrashService
{
    /// <summary>Fatal machine check. Event 19 is the corrected kind and is the watchdog's job.</summary>
    public const int FatalMachineCheckId = 18;

    private static string MarkerPath(string workRoot) => Path.Combine(workRoot, "whea_seen.json");

    private sealed record Marker(DateTime LastSeen);

    /// <summary>Newest crash already dealt with, so the same one is never offered twice.</summary>
    public static DateTime? LastAcknowledged(string workRoot)
    {
        try
        {
            string path = MarkerPath(workRoot);
            if (!File.Exists(path)) return null;

            return JsonSerializer.Deserialize<Marker>(File.ReadAllBytes(path))?.LastSeen;
        }
        catch { return null; }
    }

    public static void Acknowledge(string workRoot, DateTime newest)
    {
        try
        {
            Directory.CreateDirectory(workRoot);
            File.WriteAllBytes(
                MarkerPath(workRoot),
                JsonSerializer.SerializeToUtf8Bytes(new Marker(newest)));
        }
        catch { }
    }

    /// <summary>
    /// Fatal machine checks newer than the last acknowledged one, resolved to physical cores.
    /// Events whose APIC id maps to no known core are dropped: an unattributable crash cannot
    /// teach the tuner anything about a specific core.
    /// </summary>
    public static IReadOnlyList<WheaCrash> Unacknowledged(
        string workRoot,
        IReadOnlyList<PhysicalCore> cores,
        IReadOnlyList<WheaEvent>? events = null)
    {
        var since = LastAcknowledged(workRoot);
        events ??= WheaWatcher.ReadExisting();

        var crashes = new List<WheaCrash>();
        foreach (var e in events)
        {
            if (e.EventId != FatalMachineCheckId || e.IsWarning) continue;
            if (since is { } cutoff && e.Time <= cutoff) continue;
            if (e.ApicId is not { } apic) continue;
            if (WheaWatcher.CoreForApicId(apic, cores) is not { } core) continue;

            crashes.Add(new WheaCrash(core, e.Time, e.EventId));
        }

        // One crash usually writes one record per core involved; keep the newest per core.
        return [.. crashes
            .GroupBy(c => c.Core)
            .Select(g => g.OrderByDescending(c => c.Time).First())
            .OrderBy(c => c.Core)];
    }

    public static string Describe(
        IReadOnlyList<WheaCrash> crashes,
        IReadOnlyDictionary<int, int> marginsNow,
        bool isGerman)
    {
        if (crashes.Count == 0) return "";

        var when = crashes.Max(c => c.Time);
        string list = string.Join(", ", crashes.Select(c =>
            marginsNow.TryGetValue(c.Core, out int m)
                ? (isGerman ? $"Kern {c.Core} ({m})" : $"core {c.Core} ({m})")
                : (isGerman ? $"Kern {c.Core}" : $"core {c.Core}")));

        return isGerman
            ? $"Windows hat am {when:dd.MM.yyyy 'um' HH:mm} einen schwerwiegenden Hardwarefehler protokolliert: {list}. "
              + "Das passiert im Alltag bei Lastwechseln, die ein Dauerlast-Test nie erzeugt — ein Wert kann stundenlang Prime95 überstehen und trotzdem im Spiel zum Absturz führen.\n\n"
              + "Die genannten Werte sind die, die diese Kerne jetzt halten. Stimmen sie mit dem überein, was zum Absturzzeitpunkt anlag, sollten sie als instabil vermerkt werden — der Auto-Tuner geht dann nie wieder so tief."
            : $"Windows logged a fatal hardware error on {when:yyyy-MM-dd 'at' HH:mm}: {list}. "
              + "This happens in everyday use, on load transitions a continuous stress test never produces — a value can survive hours of Prime95 and still crash in a game.\n\n"
              + "The values shown are what those cores hold now. If they match what was applied when it crashed, they are worth recording as unstable — the auto-tuner will then never go that low again.";
    }
}
