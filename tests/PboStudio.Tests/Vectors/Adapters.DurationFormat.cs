using System.Text.Json;
using PboStudio.Core;

namespace PboStudio.Tests.Vectors;

internal static class DurationFormatAdapters
{
    [Vector("duration-format/hours")]
    public static object? Hours(JsonElement i) => DurationFormat.Hours(TimeSpan.FromSeconds(i.Long("seconds")));

    [Vector("duration-format/hoursSeconds")]
    public static object? HoursSeconds(JsonElement i) => DurationFormat.HoursSeconds(TimeSpan.FromSeconds(i.Long("seconds")));
}
