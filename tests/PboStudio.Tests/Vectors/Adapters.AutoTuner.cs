using System.Text.Json;
using PboStudio.Core;

namespace PboStudio.Tests.Vectors;

internal static class AutoTunerAdapters
{
    private static AutoTunerMode Mode(string name) => name switch
    {
        "coarse" => AutoTunerMode.Grob,
        "fine" => AutoTunerMode.Fein,
        _ => throw new ArgumentException("unknown mode " + name),
    };

    [Vector("auto-tuner/stepSize")]
    public static object? StepSize(JsonElement i) => AutoTunerService.StepSize(Mode(i.Str("mode")));

    [Vector("auto-tuner/applyGuardband")]
    public static object? ApplyGuardband(JsonElement i) =>
        AutoTunerService.ApplyGuardband(i.Int("lastKnownGood"), i.Int("guardband"));

    [Vector("auto-tuner/nextStep")]
    public static object? NextStep(JsonElement i)
    {
        var s = i.GetProperty("state");
        var state = new AutoTunerCoreState(s.IntOrNull("lastKnownGood"), s.Bool("ascending"));

        var r = AutoTunerService.CalculateNextStep(
            0, i.Int("margin"), i.Bool("passed"), Mode(i.Str("mode")),
            i.Int("maxLimit"), isGerman: false, state, i.Int("guardband"));

        return new
        {
            nextMargin = r.NextMargin,
            locked = r.CoreLocked,
            passed = r.Passed,
            state = new { lastKnownGood = r.State.LastKnownGood, ascending = r.State.Ascending },
        };
    }

    [Vector("auto-tuner/worstCaseSlots")]
    public static object? WorstCaseSlots(JsonElement i) =>
        AutoTunerService.WorstCaseSlots(
            i.Int("margin"), i.Int("maxLimit"), Mode(i.Str("mode")), i.Int("maxPasses"), i.Bool("canClimb"));
}
