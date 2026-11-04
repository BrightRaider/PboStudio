using System.Diagnostics;
using System.Text.Json;

namespace PboStudio.Core;

public sealed record SavedCoProfile(
    DateTime SavedAt,
    string CpuName,
    Dictionary<int, int> CoreMargins
);

/// <summary>
/// Manages autostart scheduled task ("PboStudioWatchdog") and CO profile persistence.
///
/// Since SMU Curve Optimizer values revert to BIOS defaults on full reboot,
/// this service allows applying saved CO margins automatically on Windows startup.
/// </summary>
public static class TaskSchedulerService
{
    private const string TaskName = "PboStudioWatchdog";

    public static string GetSavedProfilePath(string workRoot) =>
        Path.Combine(workRoot, "co_saved.json");

    public static void SaveProfile(string workRoot, string cpuName, IReadOnlyDictionary<int, int> margins)
    {
        string? tempPath = null;
        try
        {
            Directory.CreateDirectory(workRoot);
            string path = GetSavedProfilePath(workRoot);
            tempPath = path + $".tmp.{Guid.NewGuid():N}";

            var profile = new SavedCoProfile(DateTime.Now, cpuName, new Dictionary<int, int>(margins));
            byte[] bytes = JsonSerializer.SerializeToUtf8Bytes(profile, new JsonSerializerOptions { WriteIndented = true });

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

    public static SavedCoProfile? LoadSavedProfile(string workRoot)
    {
        string path = GetSavedProfilePath(workRoot);
        if (!File.Exists(path)) return null;

        try
        {
            byte[] bytes = File.ReadAllBytes(path);
            return JsonSerializer.Deserialize<SavedCoProfile>(bytes);
        }
        catch
        {
            return null;
        }
    }

    private static readonly TimeSpan SchtasksTimeout = TimeSpan.FromSeconds(20);

    public static bool IsAutostartEnabled() =>
        ProcessRunner.Run(SystemTools.SchTasks, $"/Query /TN \"{TaskName}\"", SchtasksTimeout).Ok;

    /// <summary>
    /// Creates or removes the logon task. It runs elevated at every logon, so it is only as safe as
    /// the folder the executable sits in: anything that can replace the file there gets
    /// administrator rights at the next logon without a prompt. Said in the tooltip, because the
    /// program is portable and can be anywhere.
    /// </summary>
    public static bool SetAutostart(bool enable, string exePath)
    {
        string args = enable
            ? $"/Create /TN \"{TaskName}\" /TR \"\\\"{exePath}\\\" --apply-saved\" /SC ONLOGON /RL HIGHEST /F"
            : $"/Delete /TN \"{TaskName}\" /F";

        return ProcessRunner.Run(SystemTools.SchTasks, args, SchtasksTimeout).Ok;
    }
}
