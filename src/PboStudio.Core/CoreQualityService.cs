namespace PboStudio.Core;

public enum CoreQualityRank
{
    Standard,
    GoldStar,   // Highest CPPC performance value reported by the processor
    SilverStar  // Second highest
}

/// <param name="CppcPerformance">
/// The raw CPPC "highest performance" value this core reports, or 0 when unknown. Higher means
/// the firmware rates the core as capable of boosting further.
/// </param>
public sealed record CoreQualityInfo(int CoreIndex, CoreQualityRank Rank, int CppcPerformance);

/// <summary>
/// Ranks cores by the CPPC performance value the processor itself reports.
/// <para>
/// This used to be a hard-coded guess - core 0 is gold, core 8 (or 1) is silver - presented in
/// the UI as if it had been measured. On plenty of Ryzen parts the preferred core is neither.
/// The values now come from MSR 0xC00102B3 by way of ZenStates; when they are unavailable the
/// service reports no ranking at all rather than inventing one.
/// </para>
/// </summary>
public static class CoreQualityService
{
    public static Dictionary<int, CoreQualityInfo> GetCoreQualities(
        int physicalCoreCount, IReadOnlyList<int>? cppcPerformance = null)
    {
        var result = new Dictionary<int, CoreQualityInfo>();
        for (int i = 0; i < physicalCoreCount; i++)
            result[i] = new CoreQualityInfo(i, CoreQualityRank.Standard, 0);

        if (cppcPerformance is null) return result;

        // A zero means the MSR read for that core failed; it is not a low score.
        var scored = Enumerable.Range(0, physicalCoreCount)
            .Where(i => i < cppcPerformance.Count && cppcPerformance[i] > 0)
            .Select(i => (Core: i, Score: cppcPerformance[i]))
            .ToList();

        foreach (var (core, score) in scored)
            result[core] = new CoreQualityInfo(core, CoreQualityRank.Standard, score);

        // Nothing to rank: too few readings, or firmware reporting one value for every core.
        if (scored.Count < 2) return result;
        if (scored.Select(s => s.Score).Distinct().Count() < 2) return result;

        // Ties go to the lower core index, matching how Windows breaks them.
        var ordered = scored.OrderByDescending(s => s.Score).ThenBy(s => s.Core).ToList();

        result[ordered[0].Core] = new CoreQualityInfo(ordered[0].Core, CoreQualityRank.GoldStar, ordered[0].Score);
        result[ordered[1].Core] = new CoreQualityInfo(ordered[1].Core, CoreQualityRank.SilverStar, ordered[1].Score);

        return result;
    }
}
