using System.Text.Json;
using PboStudio.Core;

namespace PboStudio.Tests.Vectors;

internal static class ApicTopologyAdapters
{
    [Vector("apic-topology/coreForApicId")]
    public static object? CoreForApicId(JsonElement i)
    {
        int count = i.GetProperty("cores").Int("count");
        int t = i.GetProperty("cores").Int("threadsPerCore");
        IReadOnlyList<PhysicalCore> cores =
        [
            .. Enumerable.Range(0, count).Select(n =>
                new PhysicalCore(n, 0, [.. Enumerable.Range(n * t, t)])),
        ];

        IReadOnlyDictionary<int, int>? map = null;
        var m = i.GetProperty("map");
        if (m.ValueKind == JsonValueKind.Object)
        {
            map = m.TryGetProperty("identity", out var n)
                ? Enumerable.Range(0, n.GetInt32()).ToDictionary(x => x, x => x)
                : m.GetProperty("pairs").EnumerateArray().ToDictionary(p => p[0].GetInt32(), p => p[1].GetInt32());
        }

        return ApicTopology.CoreForApicId(i.Int("apicId"), cores, map);
    }
}
