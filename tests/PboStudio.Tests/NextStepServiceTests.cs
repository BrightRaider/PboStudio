using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

/// <summary>
/// The single recommendation the Setup tab leads with.
///
/// It replaces a lookup table that mapped CPU generation to a profile *name*, which the UI then
/// matched back by counting shared words. Two problems: the names drifted apart, and for an X3D
/// part it named the AVX2 profile — the hottest load in the set, on the one design whose stacked
/// cache is most sensitive to temperature.
/// </summary>
public class NextStepServiceTests
{
    private const int ChipLimit = -30;
    private const string X3d = "AMD Ryzen 7 5800X3D 8-Core Processor";
    private const string Plain = "AMD Ryzen 7 7700X 8-Core Processor";

    private static IReadOnlyList<PhysicalCore> Cores(int n) =>
        [.. Enumerable.Range(0, n).Select(i => new PhysicalCore(i, (nuint)1 << i, [i]))];

    private static NextStep Recommend(
        string cpu,
        Dictionary<int, int> margins,
        Dictionary<int, CoreKnowledge>? knowledge = null,
        int cores = 8) =>
        NextStepService.Recommend(
            cpu, Cores(cores), margins, knowledge ?? [], ChipLimit, isGerman: false);

    private static Dictionary<int, int> AllAt(int value, int cores = 8) =>
        Enumerable.Range(0, cores).ToDictionary(i => i, _ => value);

    // ── first run ────────────────────────────────────────────────

    [Fact]
    public void WithNothingMeasuredItStartsWithTheSseWorkhorse()
    {
        var step = Recommend(Plain, AllAt(0));

        Assert.Equal(TuningStage.Discover, step.Stage);
        Assert.Equal(ProfileId.HeavyFfts, step.Profile);
        Assert.False(step.UseAutoTuner);
        Assert.Empty(step.Cores);
    }

    /// <summary>
    /// The regression this service exists for: an X3D must never be pointed at AVX2 as its
    /// opening move.
    /// </summary>
    [Fact]
    public void AnX3dIsNeverSentToAvx2First()
    {
        var step = Recommend(X3d, AllAt(0));

        Assert.NotEqual(ProfileId.Step2Avx2, step.Profile);
        Assert.NotEqual(ProfileId.HeavyLoad, step.Profile);
        Assert.NotEqual(ProfileId.BreakingPoint, step.Profile);
        Assert.Equal(ProfileId.HeavyFfts, step.Profile);
        Assert.Contains("X3D", step.Reason);
    }

    // ── narrowing ────────────────────────────────────────────────

    [Fact]
    public void ACoreHoldingMoreThanItCouldReachIsWorthTuning()
    {
        var margins = AllAt(-30);
        margins[3] = -18;

        var step = Recommend(Plain, margins, new Dictionary<int, CoreKnowledge>
        {
            [0] = new(BestPassed: -30),
            [3] = new(BestPassed: -18),
        });

        Assert.Equal(TuningStage.Narrow, step.Stage);
        Assert.True(step.UseAutoTuner);
        Assert.Equal([3], step.Cores);
    }

    [Fact]
    public void ACoreIsFinishedOnceItSitsAtItsOwnFloor()
    {
        // Crashed at -30, so -29 is as far as it goes — and it is already there.
        var margins = AllAt(-30);
        margins[1] = -29;

        var step = Recommend(Plain, margins, new Dictionary<int, CoreKnowledge>
        {
            [0] = new(BestPassed: -30),
            [1] = new(BestPassed: -29, WorstFailed: -30),
        });

        Assert.DoesNotContain(1, step.Cores);
    }

    /// <summary>
    /// The state this machine was actually in: six cores at the chip limit, core 0 at -20 and
    /// core 1 at -25 after it crashed the system at -30.
    /// </summary>
    [Fact]
    public void TheRealMachineGetsTheTunerPointedAtExactlyTheTwoOpenCores()
    {
        var margins = AllAt(-30);
        margins[0] = -20;
        margins[1] = -25;

        var knowledge = new Dictionary<int, CoreKnowledge>
        {
            [0] = new(BestPassed: -30),
            [1] = new(BestPassed: -25, WorstFailed: -30),
            [2] = new(BestPassed: -30),
            [3] = new(BestPassed: -30),
            [4] = new(BestPassed: -30),
            [5] = new(BestPassed: -30),
            [6] = new(BestPassed: -30),
            [7] = new(BestPassed: -30),
        };

        var step = Recommend(X3d, margins, knowledge);

        Assert.Equal(TuningStage.Narrow, step.Stage);
        Assert.True(step.UseAutoTuner);
        Assert.Equal([0, 1], step.Cores);

        // And the six that are done stay out of it — that is the whole runtime saving.
        Assert.DoesNotContain(4, step.Cores);
    }

    // ── confirming ───────────────────────────────────────────────

    [Fact]
    public void WithNothingLeftToSearchItAsksForTheOvernightRun()
    {
        var step = Recommend(Plain, AllAt(-30),
            Enumerable.Range(0, 8).ToDictionary(i => i, _ => new CoreKnowledge(BestPassed: -30)));

        Assert.Equal(TuningStage.Confirm, step.Stage);
        Assert.Equal(ProfileId.Overnight, step.Profile);
        Assert.False(step.UseAutoTuner);
    }

    [Fact]
    public void TheConfirmationWarnsAnX3dAboutTheAvx2Leg()
    {
        var step = Recommend(X3d, AllAt(-30),
            Enumerable.Range(0, 8).ToDictionary(i => i, _ => new CoreKnowledge(BestPassed: -30)));

        Assert.Contains("X3D", step.Reason);
    }

    // ── identity, not names ──────────────────────────────────────

    [Fact]
    public void EveryRecommendableProfileExistsInTheList()
    {
        var profiles = TestProfiles.For(X3d, isGerman: false);

        foreach (var id in new[] { ProfileId.HeavyFfts, ProfileId.Overnight })
            Assert.Single(profiles, p => p.Id == id);
    }

    [Fact]
    public void ProfileIdsAreUniqueSoALookupCannotBeAmbiguous()
    {
        var ids = TestProfiles.For(X3d, isGerman: false).Select(p => p.Id).ToList();
        Assert.Equal(ids.Count, ids.Distinct().Count());
    }

    [Fact]
    public void IdsSurviveTheLanguageSwitchEvenThoughEveryNameChanges()
    {
        var de = TestProfiles.For(X3d, isGerman: true);
        var en = TestProfiles.For(X3d, isGerman: false);

        Assert.Equal(de.Select(p => p.Id), en.Select(p => p.Id));
        Assert.NotEqual(de[0].Name, en[0].Name);
    }
}
