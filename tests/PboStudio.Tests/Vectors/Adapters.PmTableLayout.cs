using System.Text.Json;
using PboStudio.Core;

namespace PboStudio.Tests.Vectors;

internal static class PmTableLayoutAdapters
{
    private static PmTableLayout Layout(string name) => name switch
    {
        "Zen 3" => PmTableLayout.Zen3,
        "Zen 4" => PmTableLayout.Zen4,
        _ => throw new ArgumentException("unknown layout " + name),
    };

    [Vector("pm-table-layout/layoutFor")]
    public static object? LayoutFor(JsonElement i)
    {
        var layout = PmTableLayout.For((uint)i.Long("version"));
        return layout is null ? null : new { name = layout.Name, verified = layout.Verified };
    }

    [Vector("pm-table-layout/highestIndex")]
    public static object? HighestIndex(JsonElement i) => Layout(i.Str("layout")).HighestIndex;

    [Vector("pm-table-layout/read")]
    public static object? Read(JsonElement i)
    {
        var table = new float[i.Int("length")];
        foreach (var entry in i.GetProperty("values").EnumerateObject())
        {
            table[int.Parse(entry.Name)] = entry.Value.ValueKind switch
            {
                JsonValueKind.String when entry.Value.GetString() == "NaN" => float.NaN,
                JsonValueKind.String when entry.Value.GetString() == "Infinity" => float.PositiveInfinity,
                _ => (float)entry.Value.GetDouble(),
            };
        }

        var snap = Layout(i.Str("layout")).Read(table, 1f, 0);
        if (snap is null) return null;

        var all = new Dictionary<string, object?>
        {
            ["pptLimit"] = snap.PptLimitW,
            ["pptNow"] = snap.PptNowW,
            ["tdcLimit"] = snap.TdcLimitA,
            ["tdcNow"] = snap.TdcNowA,
            ["edcLimit"] = snap.EdcLimitA,
            ["edcNow"] = snap.EdcNowA,
            ["tempLimit"] = snap.TempLimitC,
            ["tempNow"] = snap.TempNowC,
            ["hasLimits"] = snap.HasLimits,
        };

        if (i.TryGetProperty("pick", out var pick))
        {
            var keep = pick.EnumerateArray().Select(p => p.GetString()!).ToHashSet();
            return all.Where(kv => keep.Contains(kv.Key)).ToDictionary(kv => kv.Key, kv => kv.Value);
        }
        return all;
    }
}
