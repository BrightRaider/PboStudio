using System.Text.Json;
using PboStudio.Core;

namespace PboStudio.Tests.Vectors;

internal static class LogTailAdapters
{
    [Vector("log-tail/poll")]
    public static object? Poll(JsonElement i)
    {
        string dir = Path.Combine(Path.GetTempPath(), "pbo-vector-logtail-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(dir);
        string file = Path.Combine(dir, "results.txt");

        try
        {
            long offset = 0;
            var results = new List<object>();

            foreach (var step in i.GetProperty("steps").EnumerateArray())
            {
                if (step.TryGetProperty("append", out var append)) File.AppendAllText(file, append.GetString()!);
                else if (step.TryGetProperty("set", out var set)) File.WriteAllText(file, set.GetString()!);
                else if (step.TryGetProperty("remove", out _)) { if (File.Exists(file)) File.Delete(file); }
                else if (step.TryGetProperty("poll", out _))
                {
                    var lines = LogTail.ReadNewLines(file, ref offset);
                    results.Add(new { lines, offset });
                }
                else throw new ArgumentException("unknown step " + step.GetRawText());
            }

            return results;
        }
        finally
        {
            try { Directory.Delete(dir, recursive: true); } catch { }
        }
    }
}
