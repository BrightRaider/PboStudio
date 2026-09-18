namespace PboStudio.Core;

/// <summary>
/// Which parts of the Auto tab are on screen, worked out in one place.
///
/// It was a chain of conditions spread across the method that draws the panel, and the chain
/// had a bug in it that nobody could see without starting a run: the footer's single-run button
/// was hidden on the Auto tab only while idle, so the moment a campaign started there were two
/// Cancel buttons on the same panel. A record with a test per state is cheaper to check than a
/// screenshot per state.
/// </summary>
/// <param name="ShowFooterButton">
/// The panel-wide start/stop button that belongs to the Tests and Advanced tabs. The Auto tab
/// has its own, for both starting and stopping, so this is false there in every state.
/// </param>
/// <param name="Phase">1-3 while there is something to show, 0 otherwise.</param>
public sealed record AutoPanelState(
    bool ShowSetupCard,
    bool ShowPlan,
    bool ShowResult,
    bool ShowAutoButton,
    bool ShowFooterButton,
    bool ShowSetupButton,
    bool ShowIntro,
    bool ShowEstimate,
    bool ShowStallWarning,
    int Phase);

/// <param name="OnAutoTab">Whether the Auto tab is the one showing.</param>
/// <param name="CampaignRunActive">Whether the run in flight is one the campaign started.</param>
public static class AutoPanel
{
    public static AutoPanelState Describe(
        bool supportsCurveOptimizer,
        CampaignVerdict verdict,
        int phaseNumber,
        bool finished,
        bool campaignInProgress,
        bool running,
        bool campaignRunActive,
        bool onAutoTab = true)
    {
        // Nothing else matters if the processor has no Curve Optimizer: there is no plan to
        // show and no run worth starting.
        if (!supportsCurveOptimizer)
            return new AutoPanelState(
                ShowSetupCard: true, ShowPlan: false, ShowResult: false,
                ShowAutoButton: false, ShowFooterButton: !onAutoTab, ShowSetupButton: false,
                ShowIntro: false, ShowEstimate: false, ShowStallWarning: false, Phase: 0);

        if (verdict == CampaignVerdict.NeedsSetup)
            return new AutoPanelState(
                ShowSetupCard: true, ShowPlan: false, ShowResult: false,
                ShowAutoButton: false, ShowFooterButton: !onAutoTab, ShowSetupButton: true,
                ShowIntro: false, ShowEstimate: false, ShowStallWarning: false, Phase: 0);

        if (finished)
            return new AutoPanelState(
                ShowSetupCard: false, ShowPlan: false, ShowResult: true,
                ShowAutoButton: false, ShowFooterButton: !onAutoTab, ShowSetupButton: true,
                ShowIntro: false, ShowEstimate: false, ShowStallWarning: false, Phase: 0);

        return new AutoPanelState(
            ShowSetupCard: false,
            ShowPlan: true,
            ShowResult: false,
            ShowAutoButton: true,
            ShowFooterButton: !onAutoTab,
            ShowSetupButton: true,

            // While it runs, the progress panel above says what is happening in numbers. The
            // introduction and the estimate are what somebody reads before pressing start.
            ShowIntro: !running,
            ShowEstimate: !running,

            // Decide() compares against the signature written for the run now in flight, so
            // mid-run it always reads as a stall. Only a stall found while nothing is running
            // means anything.
            ShowStallWarning: !running && verdict == CampaignVerdict.Stalled,

            Phase: running && campaignRunActive ? phaseNumber
                 : campaignInProgress || phaseNumber > 0 ? phaseNumber
                 : 0);
    }
}
