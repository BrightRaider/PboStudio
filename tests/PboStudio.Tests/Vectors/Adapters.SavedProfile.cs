using System.Text.Json;
using PboStudio.Core;

namespace PboStudio.Tests.Vectors;

internal static class SavedProfileAdapters
{
    [Vector("saved-profile/check")]
    public static object? Check(JsonElement i)
    {
        var margins = i.GetProperty("margins").EnumerateObject()
            .ToDictionary(p => int.Parse(p.Name), p => p.Value.GetInt32());
        var profile = new SavedCoProfile(DateTime.Now, i.Str("profileCpu"), margins);
        var r = SavedProfileValidator.Check(
            profile, i.Str("currentCpu"), i.Int("coreCount"), i.Int("deepest"), i.Int("shallowest"));
        return new
        {
            margins = r.Margins.ToDictionary(kv => kv.Key.ToString(), kv => kv.Value),
            problems = r.Problems,
            wrongProcessor = r.WrongProcessor,
        };
    }
}
