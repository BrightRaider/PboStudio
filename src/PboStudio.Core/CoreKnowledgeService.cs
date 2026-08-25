using System.Text.Json;

namespace PboStudio.Core;

/// <summary>
/// What has actually been observed about one core, across every run ever made.
/// </summary>
/// <param name="BestPassed">
/// The most negative margin this core has ever survived. Null when it has never passed.
/// </param>
/// <param name="WorstFailed">
/// The least negative margin this core has ever failed at — the ceiling of the bad region.
/// Everything at or below it is known bad, because a core that fails at -30 will not pass at
/// -35. Null when it has never failed.
/// </param>
public sealed record CoreKnowledge(int? BestPassed = null, int? WorstFailed = null)
{
    /// <summary>
    /// The most aggressive margin still worth testing: one point safer than the worst value
    /// that ever failed. Falls back to the chip limit when nothing has failed yet.
    /// </summary>
    public int FloorFor(int chipLimit) =>
        WorstFailed is { } bad ? Math.Max(chipLimit, bad + 1) : chipLimit;

    public bool IsKnownBad(int margin) => WorstFailed is { } bad && margin <= bad;
}

/// <summary>
/// The auto-tuner's long-term memory.
///
/// Without it the search knew only what happened since the window was opened. A core that took
/// the machine down at -30 would be walked straight back to -30 on the next run, because the
/// only record of that crash was a line in a log file no code reads. Worse, the in-memory
/// search state does not survive the crash that produced it, so the one run that learned
/// something is exactly the run that forgets it.
///
/// Every pass and every failure — including a system crash — is recorded here, and the
/// auto-tuner takes its per-core floor from it.
/// </summary>
public static class CoreKnowledgeService
{
    private static readonly JsonSerializerOptions JsonOpts = new() { WriteIndented = true };

    public static string PathFor(string workRoot) => Path.Combine(workRoot, "core_knowledge.json");

    /// <summary>
    /// The file as stored. The BIOS version travels with it because a firmware update can move
    /// stability substantially — an AGESA release changes boost behaviour and voltage curves —
    /// so knowledge gathered under the old one is evidence about a machine that no longer
    /// exists. Recorded rather than acted on: the reader decides whether to discard it.
    /// </summary>
    private sealed record Payload
    {
        public string BiosVersion { get; init; } = "";
        public Dictionary<int, CoreKnowledge> Cores { get; init; } = [];
    }

    public sealed record LoadResult(Dictionary<int, CoreKnowledge> Cores, string BiosVersion);

    public static LoadResult Load(string workRoot)
    {
        try
        {
            string path = PathFor(workRoot);
            if (!File.Exists(path)) return new LoadResult([], "");

            byte[] bytes = File.ReadAllBytes(path);

            var payload = JsonSerializer.Deserialize<Payload>(bytes, JsonOpts);
            if (payload is { Cores.Count: > 0 })
                return new LoadResult(payload.Cores, payload.BiosVersion);

            // A file written as a bare core map, before the BIOS stamp existed.
            var flat = JsonSerializer.Deserialize<Dictionary<int, CoreKnowledge>>(bytes, JsonOpts);
            return new LoadResult(flat ?? [], "");
        }
        catch
        {
            // Losing the memory is a setback, never a reason to refuse to start.
            return new LoadResult([], "");
        }
    }

    public static void Save(
        string workRoot, IReadOnlyDictionary<int, CoreKnowledge> knowledge, string biosVersion = "")
    {
        try
        {
            Directory.CreateDirectory(workRoot);
            string path = PathFor(workRoot);
            string temp = path + $".tmp.{Guid.NewGuid():N}";

            var payload = new Payload
            {
                BiosVersion = biosVersion,
                Cores = new Dictionary<int, CoreKnowledge>(knowledge),
            };

            File.WriteAllBytes(temp, JsonSerializer.SerializeToUtf8Bytes(payload, JsonOpts));
            File.Move(temp, path, overwrite: true);
        }
        catch { }
    }

    /// <summary>Forgets everything. The file is removed rather than emptied.</summary>
    public static void Reset(string workRoot)
    {
        try
        {
            string path = PathFor(workRoot);
            if (File.Exists(path)) File.Delete(path);
        }
        catch { }
    }

    /// <summary>Forgets one core, for when only that core's circumstances changed.</summary>
    public static void Reset(
        string workRoot, Dictionary<int, CoreKnowledge> knowledge, int core, string biosVersion = "")
    {
        knowledge.Remove(core);
        Save(workRoot, knowledge, biosVersion);
    }

    /// <summary>How many cores carry a value on record that failed.</summary>
    public static int CoresWithKnownBad(IReadOnlyDictionary<int, CoreKnowledge> knowledge) =>
        knowledge.Count(kv => kv.Value.WorstFailed is not null);

    /// <summary>Records that <paramref name="core"/> held up at <paramref name="margin"/>.</summary>
    public static void RecordPass(Dictionary<int, CoreKnowledge> knowledge, int core, int margin)
    {
        var existing = knowledge.GetValueOrDefault(core, new CoreKnowledge());

        // Only a more aggressive success is news.
        if (existing.BestPassed is { } best && best <= margin) return;

        knowledge[core] = existing with { BestPassed = margin };
    }

    /// <summary>
    /// Records that <paramref name="core"/> failed at <paramref name="margin"/>. A less
    /// aggressive failure widens the bad region, so it always wins over an older one.
    /// </summary>
    public static void RecordFailure(Dictionary<int, CoreKnowledge> knowledge, int core, int margin)
    {
        var existing = knowledge.GetValueOrDefault(core, new CoreKnowledge());

        if (existing.WorstFailed is { } bad && bad >= margin) return;

        // A value that now fails cannot still count as the best pass.
        int? passed = existing.BestPassed is { } p && p <= margin ? null : existing.BestPassed;

        knowledge[core] = existing with { WorstFailed = margin, BestPassed = passed };
    }

    /// <summary>Per-core floors for a run, given the chip's own limit.</summary>
    public static Dictionary<int, int> FloorsFor(
        IReadOnlyDictionary<int, CoreKnowledge> knowledge, int chipLimit) =>
        knowledge.ToDictionary(kv => kv.Key, kv => kv.Value.FloorFor(chipLimit));

    public static string Describe(CoreKnowledge k, int chipLimit, bool isGerman)
    {
        if (k.WorstFailed is null && k.BestPassed is null)
            return isGerman ? "Noch nichts über diesen Kern bekannt." : "Nothing known about this core yet.";

        var parts = new List<string>();

        if (k.BestPassed is { } good)
            parts.Add(isGerman
                ? $"bestand bereits {good}"
                : $"has passed at {good}");

        if (k.WorstFailed is { } bad)
            parts.Add(isGerman
                ? $"fiel bei {bad} durch — der Auto-Tuner geht daher nicht unter {k.FloorFor(chipLimit)}"
                : $"failed at {bad} — the auto-tuner will not go below {k.FloorFor(chipLimit)}");

        return string.Join(isGerman ? ", " : ", ", parts) + ".";
    }
}
