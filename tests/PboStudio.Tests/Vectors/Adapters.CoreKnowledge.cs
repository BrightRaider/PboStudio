using System.Text.Json;
using PboStudio.Core;

namespace PboStudio.Tests.Vectors;

internal static class CoreKnowledgeAdapters
{
    /// <summary>A JSON object of core number to knowledge.</summary>
    internal static Dictionary<int, CoreKnowledge> Knowledge(JsonElement e) =>
        e.EnumerateObject().ToDictionary(
            p => int.Parse(p.Name),
            p => new CoreKnowledge(p.Value.IntOrNull("bestPassed"), p.Value.IntOrNull("worstFailed")));

    internal static object Dump(IReadOnlyDictionary<int, CoreKnowledge> knowledge) =>
        knowledge.ToDictionary(
            kv => kv.Key.ToString(),
            kv => (object)new { bestPassed = kv.Value.BestPassed, worstFailed = kv.Value.WorstFailed });

    [Vector("core-knowledge/floorFor")]
    public static object? FloorFor(JsonElement i) =>
        new CoreKnowledge(i.IntOrNull("bestPassed"), i.IntOrNull("worstFailed")).FloorFor(i.Int("chipLimit"));

    [Vector("core-knowledge/isKnownBad")]
    public static object? IsKnownBad(JsonElement i) =>
        new CoreKnowledge(null, i.IntOrNull("worstFailed")).IsKnownBad(i.Int("margin"));

    [Vector("core-knowledge/record")]
    public static object? Record(JsonElement i)
    {
        var knowledge = new Dictionary<int, CoreKnowledge>();
        foreach (var r in i.GetProperty("results").EnumerateArray())
        {
            if (r.TryGetProperty("pass", out var p))
                CoreKnowledgeService.RecordPass(knowledge, p[0].GetInt32(), p[1].GetInt32());
            else if (r.TryGetProperty("fail", out var f))
                CoreKnowledgeService.RecordFailure(knowledge, f[0].GetInt32(), f[1].GetInt32());
            else throw new ArgumentException("unknown result " + r.GetRawText());
        }
        return Dump(knowledge);
    }

    [Vector("core-knowledge/coresWithKnownBad")]
    public static object? CoresWithKnownBad(JsonElement i) =>
        CoreKnowledgeService.CoresWithKnownBad(Knowledge(i.GetProperty("knowledge")));
}
