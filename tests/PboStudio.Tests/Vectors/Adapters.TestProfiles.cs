using System.Text.Json;
using PboStudio.Core;

namespace PboStudio.Tests.Vectors;

internal static class TestProfilesAdapters
{
    private static IReadOnlyList<TestProfile> List() => TestProfiles.For("AMD Ryzen 7 5800X3D 8-Core Processor");

    [Vector("test-profiles/ids")]
    public static object? Ids(JsonElement i) => List().Select(p => p.Id.ToString()).ToArray();

    [Vector("test-profiles/profile")]
    public static object? Profile(JsonElement i)
    {
        var id = Enum.Parse<ProfileId>(i.Str("id"));
        var p = List().First(x => x.Id == id);
        return new
        {
            minutesPerCore = p.MinutesPerCore,
            iterations = p.Iterations,
            threads = p.Threads,
            order = p.Order.ToString(),
            suspendEverySeconds = p.SuspendEverySeconds,
            suspendForSeconds = p.SuspendForSeconds,
            transient = p.Transient.ToString(),
            essential = p.Essential,
            legs = p.Phases.Select(l => new[] { l.Engine.ToString(), l.Mode.ToString(), l.Fft.ToString() }).ToArray(),
        };
    }
}
