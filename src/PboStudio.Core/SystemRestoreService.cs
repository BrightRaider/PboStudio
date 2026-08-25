using System.Diagnostics;

namespace PboStudio.Core;

/// <summary>
/// Creates a Windows restore point before the auto-tuner starts writing Curve Optimizer values.
///
/// The auto-tuner deliberately drives cores past the point where they are stable, and an
/// unstable core does not always fail cleanly: it can corrupt whatever was being written at the
/// moment it miscalculated, up to and including the registry or a system file. A restore point
/// is the difference between "reboot and carry on" and "reinstall Windows".
/// </summary>
public static class SystemRestoreService
{
    public const string RestorePointName = "PboStudio – before auto-tuner";

    /// <summary>
    /// Windows silently ignores a second restore point within 24 hours of the last one unless
    /// SystemRestorePointCreationFrequency says otherwise, so a fresh one is not always possible
    /// and the caller must not treat that as a failure.
    /// </summary>
    public static readonly TimeSpan MinimumSpacing = TimeSpan.FromHours(24);

    public sealed record RestoreResult(bool Created, string Message);

    /// <summary>
    /// True when System Protection is on for the system drive. When it is off there is nothing
    /// to create and the user has to enable it themselves - it is a machine-wide policy, not
    /// something an application should switch on behind their back.
    /// </summary>
    public static bool IsProtectionEnabled()
    {
        if (!OperatingSystem.IsWindows()) return false;

        var (ok, output) = RunPowerShell(
            "(Get-ComputerRestorePoint -ErrorAction SilentlyContinue | Measure-Object).Count -ge 0 " +
            "-and (Get-WmiObject -Namespace root\\default -Class SystemRestore -ErrorAction SilentlyContinue) -ne $null",
            TimeSpan.FromSeconds(20));

        return ok && output.Contains("True", StringComparison.OrdinalIgnoreCase);
    }

    /// <summary>When the newest existing restore point was taken, or null if there is none.</summary>
    public static DateTime? LastRestorePointTime()
    {
        if (!OperatingSystem.IsWindows()) return null;

        var (ok, output) = RunPowerShell(
            "$p = Get-ComputerRestorePoint -ErrorAction SilentlyContinue | " +
            "Sort-Object CreationTime -Descending | Select-Object -First 1; " +
            "if ($p) { $p.ConvertToDateTime($p.CreationTime).ToString('o') }",
            TimeSpan.FromSeconds(30));

        if (!ok) return null;
        string line = output.Trim();
        return DateTime.TryParse(line, null,
            System.Globalization.DateTimeStyles.RoundtripKind, out var when) ? when : null;
    }

    /// <summary>
    /// Creates the restore point. Takes tens of seconds on a spinning disk, so callers should
    /// run it off the UI thread and tell the user what is happening.
    /// </summary>
    public static RestoreResult Create(bool isGerman)
    {
        if (!OperatingSystem.IsWindows())
            return new RestoreResult(false, "System restore is only available on Windows.");

        if (!IsProtectionEnabled())
            return new RestoreResult(false, isGerman
                ? "Computerschutz ist für das Systemlaufwerk ausgeschaltet — es kann kein Wiederherstellungspunkt angelegt werden. Einschalten unter: Systemeigenschaften → Computerschutz."
                : "System protection is off for the system drive, so no restore point can be created. Turn it on under: System Properties → System Protection.");

        if (LastRestorePointTime() is { } last && DateTime.Now - last < MinimumSpacing)
            return new RestoreResult(false, isGerman
                ? $"Es existiert bereits ein Wiederherstellungspunkt von {last:dd.MM.yyyy HH:mm} — Windows legt innerhalb von 24 Stunden keinen zweiten an. Das ist in Ordnung: der vorhandene deckt den Lauf ab."
                : $"A restore point from {last:yyyy-MM-dd HH:mm} already exists — Windows will not create a second one within 24 hours. That is fine: the existing one covers this run.");

        var (ok, output) = RunPowerShell(
            $"Checkpoint-Computer -Description '{RestorePointName}' -RestorePointType 'MODIFY_SETTINGS'",
            TimeSpan.FromMinutes(3));

        return ok
            ? new RestoreResult(true, isGerman
                ? "Wiederherstellungspunkt angelegt."
                : "Restore point created.")
            : new RestoreResult(false, isGerman
                ? $"Wiederherstellungspunkt fehlgeschlagen: {Summarise(output)}"
                : $"Restore point failed: {Summarise(output)}");
    }

    private static string Summarise(string output)
    {
        string line = output.Split('\n', StringSplitOptions.RemoveEmptyEntries)
            .Select(l => l.Trim())
            .FirstOrDefault(l => l.Length > 0) ?? "unknown error";
        return line.Length > 200 ? line[..200] + "…" : line;
    }

    private static (bool Ok, string Output) RunPowerShell(string script, TimeSpan timeout)
    {
        try
        {
            var psi = new ProcessStartInfo("powershell.exe")
            {
                // -NonInteractive so a policy prompt can never block a background call.
                Arguments = $"-NoProfile -NonInteractive -ExecutionPolicy Bypass -Command \"{script.Replace("\"", "\\\"")}\"",
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true,
            };

            using var process = Process.Start(psi);
            if (process is null) return (false, "could not start powershell");

            string stdout = process.StandardOutput.ReadToEnd();
            string stderr = process.StandardError.ReadToEnd();

            if (!process.WaitForExit((int)timeout.TotalMilliseconds))
            {
                try { process.Kill(entireProcessTree: true); } catch { }
                return (false, "timed out");
            }

            return (process.ExitCode == 0, string.IsNullOrWhiteSpace(stderr) ? stdout : stderr);
        }
        catch (Exception ex)
        {
            return (false, ex.Message);
        }
    }
}
