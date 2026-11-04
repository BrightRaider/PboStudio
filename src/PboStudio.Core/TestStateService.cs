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
/// <param name="SystemRestarted">
/// True when the computer started again after the run's last sign of life - a crash or a reset.
/// False means the machine stayed up and only the run ended: the program was closed or killed.
/// </param>
public sealed record InterruptedRun(ActiveRunState State, bool SystemRestarted);

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

    /// <summary>The state of a run that did not finish, whatever ended it.</summary>
    public static ActiveRunState? DetectInterruptedRun(string workRoot)
    {
        var state = LoadState(workRoot);
        if (state is not null && state.IsActive)
            return state;

        return null;
    }

    /// <summary>
    /// A run whose file is still there because the <em>machine</em> went down, as opposed to the
    /// program being closed.
    /// <para>
    /// Any run that ends abnormally leaves its state file: a crash, but also a kill from the task
    /// manager, a crash of this program, or a window closed mid-run. Every one of them was reported
    /// at the next start as "unexpected system restart on core N", and the margin that core held was
    /// written into the tuner's memory as a failure for good - so a core that never failed was
    /// capped at a value it had not been shown to fail at. A system crash can be told apart: the
    /// computer started again after the file was last written.
    /// </para>
    /// </summary>
    /// <param name="systemStarted">When Windows last started.</param>
    public static InterruptedRun? ClassifyInterruptedRun(string workRoot, DateTime systemStarted)
    {
        var state = DetectInterruptedRun(workRoot);
        if (state is null) return null;

        // Written at every core slot, so this is the last moment the program is known to have run.
        return systemStarted > state.LastUpdateTime
            ? new InterruptedRun(state, SystemRestarted: true)
            : new InterruptedRun(state, SystemRestarted: false);
    }

    /// <summary>When Windows last started, from the uptime counter.</summary>
    public static DateTime SystemStartTime() =>
        DateTime.Now - TimeSpan.FromMilliseconds(Environment.TickCount64);
}
