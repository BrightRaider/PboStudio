using System.Text.Json;
using PboStudio.Core;

namespace PboStudio.Tests.Vectors;

internal static class CampaignAdapters
{
    private static Dictionary<int, int> Margins(JsonElement e) =>
        e.EnumerateObject().ToDictionary(p => int.Parse(p.Name), p => p.Value.GetInt32());

    private static NextStep StepOf(JsonElement s) =>
        new(ProfileId.HeavyFfts,
            Enum.Parse<TuningStage>(s.Str("stage")),
            Headline: "", Reason: "", UseAutoTuner: false,
            Cores: s.Ints("cores"),
            Action: Enum.Parse<NextStepAction>(s.Str("action")));

    [Vector("campaign/recommend")]
    public static object? Recommend(JsonElement i)
    {
        IReadOnlyList<PhysicalCore> cores = [.. i.Ints("cores").Select(n => new PhysicalCore(n, 0, [n]))];

        var step = NextStepService.Recommend(
            "AMD Ryzen 7 5800X3D 8-Core Processor", cores,
            Margins(i.GetProperty("margins")),
            CoreKnowledgeAdapters.Knowledge(i.GetProperty("knowledge")),
            i.Int("chipLimit"), isGerman: false,
            driverReady: i.Bool("driverReady"), engineReady: i.Bool("engineReady"),
            guardband: i.Int("guardband"));

        return new
        {
            stage = step.Stage.ToString(),
            profile = step.Profile.ToString(),
            useAutoTuner = step.UseAutoTuner,
            cores = step.Cores,
            action = step.Action.ToString(),
            showProfile = step.ShowProfile,
        };
    }

    [Vector("campaign/phaseNumberOf")]
    public static object? PhaseNumberOf(JsonElement i) =>
        CampaignService.PhaseNumberOf(Enum.Parse<TuningStage>(i.Str("stage")));

    [Vector("campaign/signature")]
    public static object? Signature(JsonElement i) =>
        CampaignService.Signature(
            new NextStep(ProfileId.HeavyFfts, Enum.Parse<TuningStage>(i.Str("stage")), "", "", false, i.Ints("cores")),
            Margins(i.GetProperty("margins")));

    [Vector("campaign/backOffTo")]
    public static object? BackOffTo(JsonElement i) =>
        CampaignService.BackOffTo(
            new CoreKnowledge(i.IntOrNull("bestPassed"), i.IntOrNull("worstFailed")),
            i.Int("chipLimit"), i.Int("guardband"));

    [Vector("campaign/decide")]
    public static object? Decide(JsonElement i)
    {
        var s = i.GetProperty("state");
        var state = new CampaignState
        {
            RunsDone = s.Int("runsDone"),
            LastStage = s.Str("lastStage"),
            LastSignature = s.Str("lastSignature"),
        };

        var d = CampaignService.Decide(state, StepOf(i.GetProperty("step")), Margins(i.GetProperty("margins")), i.Bool("lastRunClean"));
        return new { verdict = d.Verdict.ToString(), phaseNumber = d.PhaseNumber };
    }
}
