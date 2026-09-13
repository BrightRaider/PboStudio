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
        int cores = 8,
        bool driverReady = true,
        bool engineReady = true) =>
        NextStepService.Recommend(
            cpu, Cores(cores), margins, knowledge ?? [], ChipLimit, isGerman: false,
            driverReady, engineReady);

    // ── opening the program for the first time ───────────────────

    /// <summary>
    /// The state a genuinely new user is in: nothing installed. Recommending a test run here
    /// would sit next to a disabled start button and be useless.
    /// </summary>
    [Fact]
    public void WithNothingInstalledTheFirstStepIsTheDriver()
    {
        var step = Recommend(Plain, AllAt(0), driverReady: false, engineReady: false);

        Assert.Equal(TuningStage.InstallDriver, step.Stage);
        Assert.Equal(NextStepAction.OpenSetup, step.Action);
        Assert.False(step.ShowProfile);
    }

    [Fact]
    public void WithTheDriverInPlaceTheNextStepIsAnEngine()
    {
        var step = Recommend(Plain, AllAt(0), driverReady: true, engineReady: false);

        Assert.Equal(TuningStage.InstallEngine, step.Stage);
        Assert.Equal(NextStepAction.OpenSetup, step.Action);
        Assert.False(step.ShowProfile);
    }

    [Fact]
    public void SetupComesBeforeAnyTuningAdviceEvenWhenCoresAreKnown()
    {
        var step = Recommend(Plain, AllAt(-30),
            Enumerable.Range(0, 8).ToDictionary(i => i, _ => new CoreKnowledge(BestPassed: -30)),
            driverReady: false);

        Assert.Equal(TuningStage.InstallDriver, step.Stage);
    }

    /// <summary>
    /// The setup steps have to explain themselves: "install PawnIO" means nothing to someone
    /// who has never heard of it.
    /// </summary>
    [Fact]
    public void TheSetupStepsSayWhatTheyAreFor()
    {
        var driver = Recommend(Plain, AllAt(0), driverReady: false);
        var engine = Recommend(Plain, AllAt(0), engineReady: false);

        Assert.Contains("CO value", Explanation(driver));
        Assert.Contains("restart", Explanation(driver));

        Assert.Contains("Prime95", Explanation(engine));
        Assert.Contains("y-cruncher", Explanation(engine));
    }

    /// <summary>A first run must say that it changes nothing permanently.</summary>
    [Fact]
    public void TheFirstRunSaysItIsNotPermanent()
    {
        var step = Recommend(Plain, AllAt(0));

        Assert.Equal(TuningStage.Discover, step.Stage);
        Assert.Contains("nothing permanently", Explanation(step));
        Assert.Contains("BIOS", Explanation(step));
    }

    [Fact]
    public void TheStagesReadAsANumberedSequenceForANewcomer()
    {
        Assert.Contains("Step 1", Recommend(Plain, AllAt(0), driverReady: false).Headline);
        Assert.Contains("Step 2", Recommend(Plain, AllAt(0), engineReady: false).Headline);
        Assert.Contains("Step 3", Recommend(Plain, AllAt(0)).Headline);
    }

    /// <summary>
    /// Everything the card can tell the reader — the one line it shows plus the mechanics in
    /// its tooltip. The split between the two is a presentation decision; these tests care that
    /// the explanation exists somewhere the reader can reach it.
    /// </summary>
    private static string Explanation(NextStep step) => step.Reason + " " + step.Detail;

    private static Dictionary<int, int> AllAt(int value, int cores = 8) =>
        Enumerable.Range(0, cores).ToDictionary(i => i, _ => value);

    // ── first run ────────────────────────────────────────────────

    /// <summary>
    /// Validating a Curve Optimizer setting means alternating the two engines — a core can pass
    /// Prime95 and fail y-cruncher. The combined profile is the project's own labelled gold
    /// standard, and the recommendation has to agree with it rather than name something else.
    /// </summary>
    [Fact]
    public void WithNothingMeasuredItRunsBothEngines()
    {
        var step = Recommend(Plain, AllAt(0));

        Assert.Equal(TuningStage.Discover, step.Stage);
        Assert.Equal(ProfileId.RecommendedCombo, step.Profile);
        Assert.False(step.UseAutoTuner);
        Assert.Empty(step.Cores);

        var profile = TestProfiles.For(Plain, isGerman: false).Single(p => p.Id == step.Profile);
        Assert.True(profile.Uses(EngineKind.Prime95));
        Assert.True(profile.Uses(EngineKind.YCruncher));
    }

    /// <summary>
    /// The exception, and the reason for it: locked cores are excluded at the start of every
    /// phase, so a core the tuner settles under Prime95 would never enter the y-cruncher phase.
    /// The second engine would cost the full runtime and measure nothing.
    /// </summary>
    [Fact]
    public void TheTuningStageRunsOneEngineBecauseALockedCoreSkipsLaterPhases()
    {
        var margins = AllAt(-30);
        margins[3] = -18;

        var step = Recommend(Plain, margins, new Dictionary<int, CoreKnowledge>
        {
            [0] = new(BestPassed: -30),
            [3] = new(BestPassed: -18),
        });

        Assert.True(step.UseAutoTuner);

        var profile = TestProfiles.For(Plain, isGerman: false).Single(p => p.Id == step.Profile);
        Assert.Single(profile.Phases);
        Assert.Contains("y-cruncher", Explanation(step));
    }

    [Fact]
    public void TheConfirmationRunCoversBothEngines()
    {
        var step = Recommend(Plain, AllAt(-30),
            Enumerable.Range(0, 8).ToDictionary(i => i, _ => new CoreKnowledge(BestPassed: -30)));

        var profile = TestProfiles.For(Plain, isGerman: false).Single(p => p.Id == step.Profile);
        Assert.True(profile.Uses(EngineKind.Prime95));
        Assert.True(profile.Uses(EngineKind.YCruncher));
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

        // Whatever it picks, the first phase must be SSE rather than AVX2.
        var profile = TestProfiles.For(X3d, isGerman: false).Single(p => p.Id == step.Profile);
        Assert.Equal(Prime95Mode.Sse, profile.Mode);
        Assert.Contains("X3D", Explanation(step));
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

        Assert.Contains("X3D", Explanation(step));
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
