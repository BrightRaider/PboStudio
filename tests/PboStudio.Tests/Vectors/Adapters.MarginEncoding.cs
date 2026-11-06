using System.Text.Json;
using ZenStates.Core;

namespace PboStudio.Tests.Vectors;

internal static class MarginEncodingAdapters
{
    [Vector("margin-encoding/psmMarginArg")]
    public static object? PsmMarginArg(JsonElement i) => (long)Utils.MakePsmMarginArg(i.Int("margin"));

    // The same expression SetPsmMarginSingleCore.Execute builds for the message argument.
    [Vector("margin-encoding/setArgument")]
    public static object? SetArgument(JsonElement i) =>
        (long)(((uint)i.Int("coreMask") & 0xfff00000) | Utils.MakePsmMarginArg(i.Int("margin")));
}
