using System.Text.Json;
using PboStudio.Core;

namespace PboStudio.Tests.Vectors;

internal static class CoreQualityAdapters
{
    [Vector("core-quality/getCoreQualities")]
    public static object? GetCoreQualities(JsonElement i)
    {
        IReadOnlyList<int>? cppc = i.GetProperty("cppc").ValueKind == JsonValueKind.Null ? null : i.Ints("cppc");
        var r = CoreQualityService.GetCoreQualities(i.Int("coreCount"), cppc);
        return r.ToDictionary(kv => kv.Key.ToString(),
            kv => (object)new { rank = kv.Value.Rank.ToString(), cppc = kv.Value.CppcPerformance });
    }
}
