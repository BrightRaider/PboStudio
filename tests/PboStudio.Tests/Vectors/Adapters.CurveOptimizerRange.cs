using System.Text.Json;
using PboStudio.Core;

namespace PboStudio.Tests.Vectors;

internal static class CurveOptimizerRangeAdapters
{
    [Vector("curve-optimizer-range/effective")]
    public static object? Effective(JsonElement i) =>
        CurveOptimizerRange.Effective(
            i.Int("modelLimit"),
            i.IntOrNull("userLimit"),
            i.Ints("readFromHardware"),
            i.IntOrNull("appliedFloor"));

    [Vector("curve-optimizer-range/wasKept")]
    public static object? WasKept(JsonElement i) =>
        CurveOptimizerRange.WasKept(i.Int("requested"), i.IntOrNull("readBack"));
}
