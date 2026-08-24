using System.Text.Json;
using System.Text.Json.Serialization;

namespace PboStudio.Core;

public sealed record ActiveRunState
{
    public bool IsActive { get; init; } = true;
    public DateTime StartTime { get; init; } = DateTime.Now;
    public DateTime LastUpdateTime { get; init; } = DateTime.Now;
    public int CurrentIteration { get; init; }
    public int CurrentCore { get; init; }
    public string EngineName { get; init; } = "";
    public List<int> SelectedCores { get; init; } = [];
    public List<int> CompletedCores { get; init; } = [];
    public Dictionary<int, int> CoreMargins { get; init; } = [];
    public Dictionary<int, List<string>> Failures { get; init; } = [];
}

/// <summary>
/// Manages test state persistence on disk (runs/state.json).
///
/// If a crash or BSOD occurs mid-test, the state file remains marked as active.
/// On next application start, PboStudio reads state.json and correlates it with
/// WHEA event log records to identify the crashed core.
/// </summary>
public static class TestStateService
{
    private static readonly JsonSerializerOptions JsonOpts = new()
    {
        WriteIndented = true,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
    };

    public static string GetStateFilePath(string workRoot) =>
        Path.Combine(workRoot, "state.json");

    public static void SaveState(string workRoot, ActiveRunState state)
    {
        string? tempPath = null;
        try
        {
            Directory.CreateDirectory(workRoot);
            string path = GetStateFilePath(workRoot);
            tempPath = path + $".tmp.{Guid.NewGuid():N}";

            var updated = state with { LastUpdateTime = DateTime.Now };
            byte[] bytes = JsonSerializer.SerializeToUtf8Bytes(updated, JsonOpts);

            using (var fs = new FileStream(tempPath, FileMode.Create, FileAccess.Write, FileShare.None, 4096, FileOptions.WriteThrough))
            {
                fs.Write(bytes);
                fs.Flush(flushToDisk: true);
            }

            File.Move(tempPath, path, overwrite: true);
        }
        catch
        {
            if (tempPath is not null && File.Exists(tempPath))
            {
                try { File.Delete(tempPath); } catch { }
            }
        }
    }

    public static ActiveRunState? LoadState(string workRoot)
    {
        string path = GetStateFilePath(workRoot);
        if (!File.Exists(path)) return null;

        try
        {
            byte[] bytes = File.ReadAllBytes(path);
            return JsonSerializer.Deserialize<ActiveRunState>(bytes, JsonOpts);
        }
        catch
        {
            return null;
        }
    }

    public static void ClearState(string workRoot)
    {
        try
        {
            string path = GetStateFilePath(workRoot);
            if (File.Exists(path)) File.Delete(path);

            if (Directory.Exists(workRoot))
            {
                foreach (var tmp in Directory.EnumerateFiles(workRoot, "state.json.tmp.*"))
                {
                    try { File.Delete(tmp); } catch { }
                }
            }
        }
        catch { }
    }

    public static ActiveRunState? DetectInterruptedRun(string workRoot)
    {
        var state = LoadState(workRoot);
        if (state is not null && state.IsActive)
            return state;

        return null;
    }
}
