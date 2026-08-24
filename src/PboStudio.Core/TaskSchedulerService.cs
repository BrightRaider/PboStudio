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

    public static bool IsAutostartEnabled()
    {
        try
        {
            var psi = new ProcessStartInfo("schtasks", $"/Query /TN \"{TaskName}\"")
            {
                CreateNoWindow = true,
                UseShellExecute = false,
            };
            using var p = Process.Start(psi);
            p?.WaitForExit();
            return p?.ExitCode == 0;
        }
        catch
        {
            return false;
        }
    }

    public static bool SetAutostart(bool enable, string exePath)
    {
        try
        {
            if (enable)
            {
                // Create elevated task on logon
                string cmd = $"/Create /TN \"{TaskName}\" /TR \"\\\"{exePath}\\\" --apply-saved\" /SC ONLOGON /RL HIGHEST /F";
                var psi = new ProcessStartInfo("schtasks", cmd)
                {
                    CreateNoWindow = true,
                    UseShellExecute = false,
                };
                using var p = Process.Start(psi);
                p?.WaitForExit();
                return p?.ExitCode == 0;
            }
            else
            {
                string cmd = $"/Delete /TN \"{TaskName}\" /F";
                var psi = new ProcessStartInfo("schtasks", cmd)
                {
                    CreateNoWindow = true,
                    UseShellExecute = false,
                };
                using var p = Process.Start(psi);
                p?.WaitForExit();
                return p?.ExitCode == 0;
            }
        }
        catch
        {
            return false;
        }
    }
}
