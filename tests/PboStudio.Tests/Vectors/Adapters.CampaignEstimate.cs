using System.Text.Json;
using PboStudio.Core;

namespace PboStudio.Tests.Vectors;

internal static class CampaignEstimateAdapters
{
    [Vector("campaign-estimate/estimate")]
    public static object? Estimate(JsonElement i)
    {
        // A profile is [minutesPerCore, iterations, legs]; the extra legs are phases beyond the first.
        var profiles = new List<TestProfile>();
        foreach (var p in i.GetProperty("profiles").EnumerateObject())
        {
            var a = p.Value.EnumerateArray().Select(e => e.GetInt32()).ToArray();
            var extra = Enumerable.Range(0, a[2] - 1).Select(_ => new TestPhase()).ToList();
            profiles.Add(new TestProfile(p.Name, "", Prime95Mode.Sse, FftPreset.Huge, 1, a[0], a[1], CoreOrder.Alternate, extra)
            {
                Id = Enum.Parse<ProfileId>(p.Name),
            });
        }

        IReadOnlyList<PhysicalCore> cores = [.. i.Ints("cores").Select(n => new PhysicalCore(n, 0, [n]))];
        var margins = i.GetProperty("margins").EnumerateObject().ToDictionary(p => int.Parse(p.Name), p => p.Value.GetInt32());

        var e = CampaignEstimator.Estimate(
            profiles, cores, margins,
            CoreKnowledgeAdapters.Knowledge(i.GetProperty("knowledge")),
            i.Int("chipLimit"), i.Int("guardband"), i.Int("fromPhase"));

        return new
        {
            total = e.Total.TotalMinutes,
            perPhase = e.PerPhase.Select(p => new object[] { p.Phase, p.Span.TotalMinutes }).ToArray(),
        };
    }
}
