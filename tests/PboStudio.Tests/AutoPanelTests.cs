using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

/// <summary>
/// One test per state of the Auto tab, because the alternative is one screenshot per state and
/// somebody having to look at every element by hand.
/// </summary>
public class AutoPanelTests
{
    private static AutoPanelState Idle() => AutoPanel.Describe(
        supportsCurveOptimizer: true, CampaignVerdict.Run, phaseNumber: 1,
        finished: false, campaignInProgress: false, running: false, campaignRunActive: false);

    private static AutoPanelState Running(int phase = 2) => AutoPanel.Describe(
        supportsCurveOptimizer: true, CampaignVerdict.Stalled, phaseNumber: phase,
        finished: false, campaignInProgress: true, running: true, campaignRunActive: true);

    // ── the bug this file exists for ──────────────────────────────

    /// <summary>
    /// The footer's single-run button was hidden on the Auto tab only while idle, so starting a
    /// campaign put two Cancel buttons on the same panel — one from the campaign, one from the
    /// run underneath it.
    /// </summary>
    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void TheAutoTabNeverShowsTheFooterButton(bool running)
    {
        var state = running ? Running() : Idle();

        Assert.False(state.ShowFooterButton);
        Assert.True(state.ShowAutoButton);
    }

    [Fact]
    public void TheOtherTabsDoShowTheFooterButton()
    {
        var state = AutoPanel.Describe(
            supportsCurveOptimizer: true, CampaignVerdict.Run, 1,
            finished: false, campaignInProgress: false, running: false,
            campaignRunActive: false, onAutoTab: false);

        Assert.True(state.ShowFooterButton);
    }

    // ── what belongs on screen while it runs ──────────────────────

    /// <summary>
    /// The introduction and the estimate are read before pressing start. During a run the panel
    /// above carries the elapsed time and what is left, and the resume line in particular read
    /// "Interrupted — nothing is lost" over a plainly running test.
    /// </summary>
    [Fact]
    public void TheProseIsHiddenWhileSomethingIsRunning()
    {
        var state = Running();

        Assert.False(state.ShowIntro);
        Assert.False(state.ShowEstimate);
        Assert.True(state.ShowPlan);
    }

    [Fact]
    public void TheProseIsThereBeforeAnythingStarts()
    {
        var state = Idle();

        Assert.True(state.ShowIntro);
        Assert.True(state.ShowEstimate);
    }

    /// <summary>
    /// The stall check compares against the signature written for the run now in flight, so it
    /// always matches mid-run. Reporting "the last run changed nothing" while that run is still
    /// going would be wrong every single time.
    /// </summary>
    [Fact]
    public void AStallIsNeverReportedWhileARunIsInFlight()
    {
        Assert.False(Running().ShowStallWarning);
    }

    [Fact]
    public void AStallIsReportedOnceNothingIsRunning()
    {
        var state = AutoPanel.Describe(
            supportsCurveOptimizer: true, CampaignVerdict.Stalled, phaseNumber: 2,
            finished: false, campaignInProgress: true, running: false, campaignRunActive: false);

        Assert.True(state.ShowStallWarning);
    }

    [Fact]
    public void ThePhaseShownIsTheOneInFlight()
    {
        Assert.Equal(3, Running(phase: 3).Phase);
    }

    // ── the states where there is nothing to run ──────────────────

    /// <summary>
    /// Curve Optimizer arrived with Zen 3. On an older part the plan, the button and the setup
    /// button are all beside the point — only the explanation belongs on screen.
    /// </summary>
    [Fact]
    public void AProcessorWithoutTheFeatureGetsAnExplanationAndNoButtons()
    {
        var state = AutoPanel.Describe(
            supportsCurveOptimizer: false, CampaignVerdict.Run, 1,
            finished: false, campaignInProgress: false, running: false, campaignRunActive: false);

        Assert.True(state.ShowSetupCard);
        Assert.False(state.ShowSetupButton);
        Assert.False(state.ShowAutoButton);
        Assert.False(state.ShowPlan);
        Assert.False(state.ShowFooterButton);
    }

    [Fact]
    public void MissingSetupGetsTheCardAndTheAssistantButton()
    {
        var state = AutoPanel.Describe(
            supportsCurveOptimizer: true, CampaignVerdict.NeedsSetup, 0,
            finished: false, campaignInProgress: false, running: false, campaignRunActive: false);

        Assert.True(state.ShowSetupCard);
        Assert.True(state.ShowSetupButton);
        Assert.False(state.ShowAutoButton);
        Assert.False(state.ShowPlan);
    }

    /// <summary>
    /// Finished is the one state with values worth reading. Leaving a Start button next to them
    /// would invite somebody to throw away a weekend's result by pressing it.
    /// </summary>
    [Fact]
    public void AFinishedCampaignShowsTheValuesAndNoStartButton()
    {
        var state = AutoPanel.Describe(
            supportsCurveOptimizer: true, CampaignVerdict.Done, 3,
            finished: true, campaignInProgress: false, running: false, campaignRunActive: false);

        Assert.True(state.ShowResult);
        Assert.False(state.ShowPlan);
        Assert.False(state.ShowAutoButton);
        Assert.False(state.ShowFooterButton);
    }

    /// <summary>Exactly one of the three cards is ever on screen.</summary>
    [Theory]
    [InlineData(false, CampaignVerdict.Run, false)]
    [InlineData(true, CampaignVerdict.Run, false)]
    [InlineData(true, CampaignVerdict.NeedsSetup, false)]
    [InlineData(true, CampaignVerdict.Done, true)]
    [InlineData(true, CampaignVerdict.Stalled, false)]
    public void ExactlyOneCardIsVisibleInEveryState(bool supportsCo, CampaignVerdict verdict, bool finished)
    {
        foreach (bool running in new[] { false, true })
        {
            var state = AutoPanel.Describe(
                supportsCo, verdict, 2, finished,
                campaignInProgress: true, running, campaignRunActive: running);

            int visible = (state.ShowSetupCard ? 1 : 0) + (state.ShowPlan ? 1 : 0) + (state.ShowResult ? 1 : 0);
            Assert.Equal(1, visible);
        }
    }

    /// <summary>And there is never a state with no way to start and no way to stop.</summary>
    [Theory]
    [InlineData(CampaignVerdict.Run)]
    [InlineData(CampaignVerdict.Stalled)]
    public void ARunnableStateAlwaysOffersExactlyOneButton(CampaignVerdict verdict)
    {
        foreach (bool running in new[] { false, true })
        {
            var state = AutoPanel.Describe(
                supportsCurveOptimizer: true, verdict, 2, finished: false,
                campaignInProgress: true, running, campaignRunActive: running);

            Assert.True(state.ShowAutoButton);
            Assert.False(state.ShowFooterButton);
        }
    }
}
