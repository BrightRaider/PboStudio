namespace PboStudio.Core;

/// <param name="Total">Everything still to do, from the phase the campaign is on to the end.</param>
/// <param name="PerPhase">What each remaining phase costs, in order, for the breakdown.</param>
public sealed record CampaignEstimate(TimeSpan Total, IReadOnlyList<(int Phase, TimeSpan Span)> PerPhase);

/// <summary>
/// How long the whole campaign still has to run, from what this machine currently holds.
///
/// The Auto tab used to say "a night to a weekend, depending on the processor", which is true
/// and useless: a processor with six cores already at the chip limit and two to search is an
/// evening, and the same words covered both. The numbers are all there — what each core holds,
/// what it is allowed to reach, and what each phase's profile runs — so there is no reason to
/// guess in prose.
/// </summary>
public static class CampaignEstimator
{
    /// <summary>
    /// The profile each phase runs. It has to match what <see cref="NextStepService"/> hands
    /// back, or the estimate describes a different campaign from the one that will happen.
    /// </summary>
    public static ProfileId ProfileForPhase(int phase) => phase switch
    {
        1 => ProfileId.RecommendedCombo,
        2 => ProfileId.HeavyFfts,
        _ => ProfileId.Overnight,
    };

    /// <param name="fromPhase">The phase the campaign is on; earlier ones are already done.</param>
    public static CampaignEstimate Estimate(
        IReadOnlyList<TestProfile> profiles,
        IReadOnlyList<PhysicalCore> cores,
        IReadOnlyDictionary<int, int> margins,
        IReadOnlyDictionary<int, CoreKnowledge> knowledge,
        int chipLimit,
        int guardband,
        int fromPhase)
    {
        var perPhase = new List<(int, TimeSpan)>();

        for (int phase = Math.Max(1, fromPhase); phase <= CampaignService.TotalPhases; phase++)
        {
            var profile = profiles.FirstOrDefault(p => p.Id == ProfileForPhase(phase));
            if (profile is null) continue;

            perPhase.Add((phase, PhaseSpan(profile, phase, cores, margins, knowledge, chipLimit, guardband)));
        }

        return new CampaignEstimate(
            perPhase.Aggregate(TimeSpan.Zero, (sum, p) => sum + p.Item2),
            perPhase);
    }

    private static TimeSpan PhaseSpan(
        TestProfile profile,
        int phase,
        IReadOnlyList<PhysicalCore> cores,
        IReadOnlyDictionary<int, int> margins,
        IReadOnlyDictionary<int, CoreKnowledge> knowledge,
        int chipLimit,
        int guardband)
    {
        double legs = profile.Phases.Count;

        // Phases one and three test every core a fixed number of times.
        if (phase != 2)
            return TimeSpan.FromMinutes(cores.Count * profile.MinutesPerCore * profile.Iterations * legs);

        // The search only visits cores with room left, and each of those costs as many slots as
        // the walk down to its own floor and back takes. That is the difference between an
        // evening and a night, and it is why the estimate cannot be a constant.
        int slots = 0;
        foreach (var core in cores)
        {
            int held = margins.TryGetValue(core.Index, out int m) ? m : 0;
            int floor = knowledge.TryGetValue(core.Index, out var k) ? k.FloorFor(chipLimit) : chipLimit;
            int target = AutoTunerService.ApplyGuardband(floor, guardband);

            if (held <= target) continue;

            slots += AutoTunerService.WorstCaseSlots(held, floor, AutoTunerMode.Fein, int.MaxValue);
        }

        return TimeSpan.FromMinutes(slots * profile.MinutesPerCore * legs);
    }
}
