using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

/// <summary>
/// The one button that runs the whole thing.
///
/// Tuning a Curve Optimizer is three runs, and until now the program asked the reader to come
/// back between them and press a button again. These cover the part that makes chaining them
/// safe: when to stop, and how to notice that nothing is moving.
/// </summary>
public class CampaignServiceTests
{
    private const int ChipLimit = -30;
    private const string Cpu = "AMD Ryzen 7 5800X3D 8-Core Processor";

    private static IReadOnlyList<PhysicalCore> Cores(int n = 8) =>
        [.. Enumerable.Range(0, n).Select(i => new PhysicalCore(i, (nuint)1 << i, [i]))];

    private static Dictionary<int, int> AllAt(int value, int cores = 8) =>
        Enumerable.Range(0, cores).ToDictionary(i => i, _ => value);

    private static NextStep Step(
        Dictionary<int, int> margins,
        Dictionary<int, CoreKnowledge>? knowledge = null,
        bool driverReady = true) =>
        NextStepService.Recommend(
            Cpu, Cores(), margins, knowledge ?? [], ChipLimit, isGerman: false,
            driverReady, engineReady: true);

    // ── the three phases, in order ────────────────────────────────

    [Fact]
    public void AFreshMachineStartsAtPhaseOne()
    {
        var margins = AllAt(0);
        var decision = CampaignService.Decide(new CampaignState(), Step(margins), margins, lastRunClean: true);

        Assert.Equal(CampaignVerdict.Run, decision.Verdict);
        Assert.Equal(TuningStage.Discover, decision.Step!.Stage);
        Assert.Equal(1, decision.PhaseNumber);
    }

    [Fact]
    public void OnceSomethingIsKnownTheSearchIsPhaseTwo()
    {
        var margins = AllAt(-30);
        margins[3] = -18;

        var decision = CampaignService.Decide(
            new CampaignState { Active = true, RunsDone = 1, LastStage = nameof(TuningStage.Discover) },
            Step(margins, new Dictionary<int, CoreKnowledge>
            {
                [0] = new(BestPassed: -30),
                [3] = new(BestPassed: -18),
            }),
            margins,
            lastRunClean: true);

        Assert.Equal(CampaignVerdict.Run, decision.Verdict);
        Assert.Equal(TuningStage.Narrow, decision.Step!.Stage);
        Assert.Equal(2, decision.PhaseNumber);
    }

    [Fact]
    public void WithNothingLeftToSearchItProvesTheResult()
    {
        var margins = AllAt(-30);
        var knowledge = Enumerable.Range(0, 8).ToDictionary(i => i, _ => new CoreKnowledge(BestPassed: -30));

        var decision = CampaignService.Decide(
            new CampaignState { Active = true, RunsDone = 2, LastStage = nameof(TuningStage.Narrow) },
            Step(margins, knowledge),
            margins,
            lastRunClean: true);

        Assert.Equal(CampaignVerdict.Run, decision.Verdict);
        Assert.Equal(TuningStage.Confirm, decision.Step!.Stage);
        Assert.Equal(3, decision.PhaseNumber);
    }

    // ── stopping ──────────────────────────────────────────────────

    /// <summary>
    /// The whole point of the campaign: a clean confirmation run is the end, not another
    /// prompt. Without this it would start the overnight run again every morning.
    /// </summary>
    [Fact]
    public void AConfirmationRunThatPassesEndsTheCampaign()
    {
        var margins = AllAt(-30);
        var knowledge = Enumerable.Range(0, 8).ToDictionary(i => i, _ => new CoreKnowledge(BestPassed: -30));

        var decision = CampaignService.Decide(
            new CampaignState { Active = true, RunsDone = 3, LastStage = nameof(TuningStage.Confirm) },
            Step(margins, knowledge),
            margins,
            lastRunClean: true);

        Assert.Equal(CampaignVerdict.Done, decision.Verdict);
    }

    /// <summary>
    /// A confirmation run that found something is not the end. The core that failed gets backed
    /// off, which puts it back above its floor, and the search picks it up again.
    /// </summary>
    [Fact]
    public void AConfirmationRunThatFailsSendsTheCoreBackToTheSearch()
    {
        var knowledge = Enumerable.Range(0, 8)
            .ToDictionary(i => i, _ => new CoreKnowledge(BestPassed: -30));
        knowledge[4] = new CoreKnowledge(WorstFailed: -30);

        // Core 4 failed at -30, so it is backed off before the next decision is taken.
        var margins = AllAt(-30);
        margins[4] = CampaignService.BackOffTo(knowledge[4], ChipLimit, guardband: 3);

        var decision = CampaignService.Decide(
            new CampaignState { Active = true, RunsDone = 3, LastStage = nameof(TuningStage.Confirm) },
            Step(margins, knowledge),
            margins,
            lastRunClean: false);

        Assert.Equal(CampaignVerdict.Run, decision.Verdict);
        Assert.Equal(TuningStage.Narrow, decision.Step!.Stage);
        Assert.Equal([4], decision.Step.Cores);
    }

    [Fact]
    public void BackingOffLandsAboveTheValueThatFailed()
    {
        var failed = new CoreKnowledge(WorstFailed: -30);

        // Floor is -29; three points of headroom on top of that.
        Assert.Equal(-26, CampaignService.BackOffTo(failed, ChipLimit, guardband: 3));

        // And never past zero, whatever the guardband says.
        Assert.Equal(0, CampaignService.BackOffTo(new CoreKnowledge(WorstFailed: -1), ChipLimit, guardband: 9));
    }

    /// <summary>
    /// Every failure raises a core's floor and every pass lowers what it holds, so a run that
    /// does neither has learned nothing — and so would the next one. Repeating it forever is
    /// worse than stopping and saying so.
    /// </summary>
    [Fact]
    public void ARunThatChangesNothingStopsTheCampaignRatherThanRepeating()
    {
        var margins = AllAt(-30);
        margins[3] = -18;
        var knowledge = new Dictionary<int, CoreKnowledge>
        {
            [0] = new(BestPassed: -30),
            [3] = new(BestPassed: -18),
        };

        var step = Step(margins, knowledge);
        var state = new CampaignState
        {
            Active = true,
            RunsDone = 2,
            LastStage = nameof(TuningStage.Narrow),
            LastSignature = CampaignService.Signature(step, margins),
        };

        Assert.Equal(CampaignVerdict.Stalled, CampaignService.Decide(state, step, margins, true).Verdict);
    }

    /// <summary>The same cores at different values is progress, not a stall.</summary>
    [Fact]
    public void LoweringACoreCountsAsProgress()
    {
        var before = AllAt(-30);
        before[3] = -18;
        var knowledge = new Dictionary<int, CoreKnowledge>
        {
            [0] = new(BestPassed: -30),
            [3] = new(BestPassed: -18),
        };

        var state = new CampaignState
        {
            Active = true,
            RunsDone = 2,
            LastStage = nameof(TuningStage.Narrow),
            LastSignature = CampaignService.Signature(Step(before, knowledge), before),
        };

        var after = AllAt(-30);
        after[3] = -24;
        knowledge[3] = new CoreKnowledge(BestPassed: -24);

        Assert.Equal(CampaignVerdict.Run, CampaignService.Decide(state, Step(after, knowledge), after, true).Verdict);
    }

    /// <summary>The first run can never be a stall: there is nothing to have repeated yet.</summary>
    [Fact]
    public void TheFirstRunIsNeverAStall()
    {
        var margins = AllAt(0);
        var step = Step(margins);
        var state = new CampaignState { LastSignature = CampaignService.Signature(step, margins) };

        Assert.Equal(CampaignVerdict.Run, CampaignService.Decide(state, step, margins, true).Verdict);
    }

    // ── nothing can run at all ────────────────────────────────────

    [Fact]
    public void WithoutTheDriverTheCampaignAsksForSetupInsteadOfStarting()
    {
        var margins = AllAt(0);
        var decision = CampaignService.Decide(
            new CampaignState(), Step(margins, driverReady: false), margins, true);

        Assert.Equal(CampaignVerdict.NeedsSetup, decision.Verdict);
        Assert.Null(decision.Step);
    }

    // ── surviving the reboot the middle phase is designed to cause ─

    [Fact]
    public void ACampaignSurvivesBeingWrittenAndReadBack()
    {
        string dir = Path.Combine(Path.GetTempPath(), "pbo-campaign-" + Guid.NewGuid().ToString("N"));
        try
        {
            var state = new CampaignState
            {
                Active = true,
                RunsDone = 2,
                LastStage = nameof(TuningStage.Narrow),
                LastSignature = "Narrow|0:-20,1:-25",
            };

            CampaignService.Save(dir, state);
            var back = CampaignService.Load(dir);

            Assert.True(back.InProgress);
            Assert.Equal(2, back.RunsDone);
            Assert.Equal(nameof(TuningStage.Narrow), back.LastStage);
            Assert.Equal("Narrow|0:-20,1:-25", back.LastSignature);
        }
        finally
        {
            if (Directory.Exists(dir)) Directory.Delete(dir, recursive: true);
        }
    }

    [Fact]
    public void NoCampaignFileMeansNoCampaign()
    {
        var state = CampaignService.Load(Path.Combine(Path.GetTempPath(), "pbo-absent-" + Guid.NewGuid().ToString("N")));

        Assert.False(state.Active);
        Assert.False(state.InProgress);
    }

    [Fact]
    public void AFinishedCampaignIsNotInProgress()
    {
        Assert.False(new CampaignState { Active = true, Finished = true }.InProgress);
    }
}
