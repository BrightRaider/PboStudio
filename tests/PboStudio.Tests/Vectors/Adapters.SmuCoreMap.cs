using System.Text.Json;
using PboStudio.Core;

namespace PboStudio.Tests.Vectors;

internal static class SmuCoreMapAdapters
{
    [Vector("smu-core-map/build")]
    public static object? Build(JsonElement i)
    {
        IReadOnlyList<uint>? disabled = i.GetProperty("disabled").ValueKind == JsonValueKind.Null
            ? null
            : i.GetProperty("disabled").EnumerateArray().Select(x => x.GetUInt32()).ToList();

        var map = SmuCoreMap.Build(disabled, i.Int("windowsCores"));
        return map?.Select(s => new[] { s.Ccd, s.Slot }).ToArray();
    }

    [Vector("smu-core-map/fallback")]
    public static object? Fallback(JsonElement i)
    {
        var slot = SmuCoreMap.Fallback(i.Int("core"), i.Int("ccdCount"), i.Int("windowsCores"));
        return slot is null ? null : new[] { slot.Ccd, slot.Slot };
    }
}
