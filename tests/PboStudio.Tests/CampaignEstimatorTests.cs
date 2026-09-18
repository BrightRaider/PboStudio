using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

/// <summary>
/// The number that belongs above the start button.
///
/// The Auto tab said "a night to a weekend, depending on the processor" — true, and useless. A
/// machine with six cores already at the chip limit and two left to search is an evening; the
/// same sentence covered both. Everything needed to say which is already on screen.
/// </summary>
public class CampaignEstimatorTests
{
    private const int ChipLimit = -30;
    private const int Guardband = 3;
    private const string Cpu = "AMD Ryzen 7 5800X3D 8-Core Processor";

    private static IReadOnlyList<TestProfile> Profiles => TestProfiles.For(Cpu, isGerman: false);

    private static IReadOnlyList<PhysicalCore> Cores(int n = 8) =>
        [.. Enumerable.Range(0, n).Select(i => new PhysicalCore(i, (nuint)1 << i, [i]))];

    private static CampaignEstimate Estimate(
        Dictionary<int, int> margins,
        Dictionary<int, CoreKnowledge>? knowledge = null,
        int fromPhase = 1) =>
        CampaignEstimator.Estimate(
            Profiles, Cores(), margins, knowledge ?? [], ChipLimit, Guardband, fromPhase);

    private static Dictionary<int, int> AllAt(int v) =>
        Enumerable.Range(0, 8).ToDictionary(i => i, _ => v);

    /// <summary>
    /// The estimate has to describe the campaign that will actually happen, so the profile it
    /// costs for each phase has to be the one NextStepService hands back for that phase.
    /// </summary>
    [Fact]
    public void EachPhaseIsCostedWithTheProfileThatPhaseActuallyRuns()
    {
        Assert.Equal(ProfileId.RecommendedCombo, CampaignEstimator.ProfileForPhase(1));
        Assert.Equal(ProfileId.HeavyFfts, CampaignEstimator.ProfileForPhase(2));
        Assert.Equal(ProfileId.Overnight, CampaignEstimator.ProfileForPhase(3));

        // And all three exist, or a phase would silently cost nothing.
        foreach (int phase in new[] { 1, 2, 3 })
            Assert.Single(Profiles, p => p.Id == CampaignEstimator.ProfileForPhase(phase));
    }

    [Fact]
    public void AFreshMachineIsCostedForAllThreePhases()
    {
        var estimate = Estimate(AllAt(0));

        Assert.Equal([1, 2, 3], estimate.PerPhase.Select(p => p.Phase));
        Assert.True(estimate.Total > TimeSpan.FromHours(1));
    }

    [Fact]
    public void PhasesAlreadyDoneAreNotCounted()
    {
        var margins = AllAt(0);

        var whole = Estimate(margins, fromPhase: 1);
        var rest = Estimate(margins, fromPhase: 3);

        Assert.Single(rest.PerPhase);
        Assert.Equal(3, rest.PerPhase[0].Phase);
        Assert.True(rest.Total < whole.Total);
    }

    /// <summary>
    /// The point of estimating rather than guessing: a machine with two cores left to search
    /// must come out well under one where all eight are open.
    /// </summary>
    [Fact]
    public void FewerCoresLeftToSearchMeansAShorterCampaign()
    {
        var knowledge = Enumerable.Range(0, 8)
            .ToDictionary(i => i, _ => new CoreKnowledge(BestPassed: -30));

        var everythingOpen = Estimate(AllAt(0), knowledge, fromPhase: 2);

        // Six cores parked at their target, two still holding something milder.
        var mostlyDone = AllAt(-27);
        mostlyDone[0] = -10;
        mostlyDone[1] = -12;
        var twoOpen = Estimate(mostlyDone, knowledge, fromPhase: 2);

        Assert.True(twoOpen.Total < everythingOpen.Total,
            $"two open cores estimated at {twoOpen.Total}, eight at {everythingOpen.Total}");
    }

    /// <summary>
    /// A core that has already crashed cannot be searched all the way to the chip limit — its
    /// floor sits above it — so it costs less than a core with the full range still open.
    /// </summary>
    [Fact]
    public void AKnownCrashShortensTheSearchItImplies()
    {
        var margins = AllAt(-5);

        var wideOpen = Estimate(margins, new Dictionary<int, CoreKnowledge>(), fromPhase: 2);

        var narrowed = Estimate(margins, Enumerable.Range(0, 8)
            .ToDictionary(i => i, _ => new CoreKnowledge(WorstFailed: -12)), fromPhase: 2);

        Assert.True(narrowed.Total < wideOpen.Total);
    }

    /// <summary>
    /// With every core already parked where the search would leave it, phase two has nothing to
    /// do and must cost nothing rather than a default.
    /// </summary>
    [Fact]
    public void NothingLeftToSearchCostsNothingForThatPhase()
    {
        var knowledge = Enumerable.Range(0, 8)
            .ToDictionary(i => i, _ => new CoreKnowledge(BestPassed: -30));

        var estimate = Estimate(AllAt(-27), knowledge, fromPhase: 2);

        Assert.Equal(TimeSpan.Zero, estimate.PerPhase.Single(p => p.Phase == 2).Span);
        Assert.True(estimate.PerPhase.Single(p => p.Phase == 3).Span > TimeSpan.Zero);
    }

    /// <summary>
    /// The confirmation run chains four load types, so it costs four times what its per-core
    /// minutes suggest. Counting one leg would understate the longest phase by a factor of four.
    /// </summary>
    [Fact]
    public void AProfileWithSeveralLegsIsCostedForAllOfThem()
    {
        var overnight = Profiles.Single(p => p.Id == ProfileId.Overnight);
        Assert.True(overnight.Phases.Count >= 4);

        var estimate = Estimate(AllAt(-27), Enumerable.Range(0, 8)
            .ToDictionary(i => i, _ => new CoreKnowledge(BestPassed: -30)), fromPhase: 3);

        var expected = TimeSpan.FromMinutes(
            8 * overnight.MinutesPerCore * overnight.Iterations * overnight.Phases.Count);

        Assert.Equal(expected, estimate.Total);
    }

    [Fact]
    public void MoreCoresCostMore()
    {
        var eight = CampaignEstimator.Estimate(
            Profiles, Cores(8), AllAt(0), new Dictionary<int, CoreKnowledge>(),
            ChipLimit, Guardband, fromPhase: 1);

        var four = CampaignEstimator.Estimate(
            Profiles, Cores(4), Enumerable.Range(0, 4).ToDictionary(i => i, _ => 0),
            new Dictionary<int, CoreKnowledge>(), ChipLimit, Guardband, fromPhase: 1);

        Assert.True(four.Total < eight.Total);
    }
}

/// <summary>
/// The estimate against this machine's actual state, because "18 hours" was the first number
/// the tool ever showed and it was wrong in a way arithmetic can settle.
/// </summary>
public class RealMachineEstimateTests
{
    private const int ChipLimit = -30;
    private const int Guardband = 3;
    private const string Cpu = "AMD Ryzen 7 5800X3D 8-Core Processor";

    private static IReadOnlyList<PhysicalCore> Cores() =>
        [.. Enumerable.Range(0, 8).Select(i => new PhysicalCore(i, (nuint)1 << i, [i]))];

    // What the processor holds, and what is on record about it.
    private static readonly Dictionary<int, int> Margins = new()
    {
        [0] = -16, [1] = -25, [2] = -30, [3] = -30,
        [4] = -26, [5] = -30, [6] = -30, [7] = -30,
    };

    private static readonly Dictionary<int, CoreKnowledge> Knowledge = new()
    {
        [0] = new(BestPassed: -20, WorstFailed: -20),
        [1] = new(BestPassed: -25, WorstFailed: -30),
        [2] = new(BestPassed: -30),
        [3] = new(BestPassed: -30),
        [4] = new(WorstFailed: -28),
        [5] = new(BestPassed: -30),
        [6] = new(BestPassed: -30),
        [7] = new(BestPassed: -30),
    };

    private static CampaignEstimate Estimate() => CampaignEstimator.Estimate(
        TestProfiles.For(Cpu, isGerman: false), Cores(), Margins, Knowledge,
        ChipLimit, Guardband, fromPhase: 2);

    /// <summary>
    /// Seven of the eight cores are already parked where the search would leave them. Only core
    /// 1 has room, it holds -25 with -25 on record as passing, and its floor is -29 — four steps
    /// to try and then it is finished.
    ///
    /// This was costed at twenty-six slots, because the estimate assumed the search might have
    /// to walk the core from -25 all the way back to 0. It cannot: a failure returns to the
    /// value already on record and locks there. Half an hour was quoted as two and a half.
    /// </summary>
    [Fact]
    public void TheSearchPhaseCostsTheDescentOnlyWhenTheCoreHasAValueOnRecord()
    {
        var search = Estimate().PerPhase.Single(p => p.Phase == 2).Span;

        Assert.Equal(TimeSpan.FromMinutes(30), search);
    }

    /// <summary>
    /// And the rest of the estimate is not a mistake: the confirmation run really is sixteen
    /// hours. Eight cores, ten minutes each, three passes, across four load types. Shortening it
    /// is a decision about how much proof is wanted, not a bug to fix.
    /// </summary>
    [Fact]
    public void TheConfirmationPhaseIsWhatTheProfileActuallyCosts()
    {
        Assert.Equal(TimeSpan.FromHours(16), Estimate().PerPhase.Single(p => p.Phase == 3).Span);
    }

    [Fact]
    public void WhichPutsTheWholeThingAtUnderSeventeenHours()
    {
        Assert.InRange(Estimate().Total, TimeSpan.FromHours(16), TimeSpan.FromHours(17));
    }
}
