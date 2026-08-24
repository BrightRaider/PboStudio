using System.Management;

namespace PboStudio.Core;

public sealed record RamAuditResult(
    int ConfiguredSpeedMhz,
    int MaxSpeedMhz,
    bool IsXmpExpoActive,
    string Advice
);

public sealed record BiosAuditResult(
    string BiosVendor,
    string BiosVersion,
    DateTime? ReleaseDate,
    bool IsOutdated,
    string BiosAdvice
);

public sealed record SystemHealthAudit(
    RamAuditResult Ram,
    BiosAuditResult Bios,
    int OverallScore,
    IReadOnlyList<string> ActionItems
);

/// <summary>
/// Platform & BIOS Audit Service.
/// Checks EXPO/XMP memory profile activation, AGESA BIOS release date, and overall system tuning score.
/// </summary>
public static class BiosAuditService
{
    public static SystemHealthAudit RunFullAudit()
    {
        var ram = AuditRam();
        var bios = AuditBios();

        int score = 100;
        var items = new List<string>();

        if (!ram.IsXmpExpoActive)
        {
            score -= 25;
            items.Add(LocalizationService.Pick(
                "⚠️ EXPO / XMP ist im BIOS nicht aktiviert — dein RAM läuft auf der gedrosselten JEDEC-Grundfrequenz.",
                "⚠️ EXPO / XMP is not enabled in the BIOS — your RAM is running at the throttled JEDEC base speed."));
        }
        else
        {
            items.Add(LocalizationService.Pick(
                $"✅ EXPO / XMP aktiv: DDR-{ram.ConfiguredSpeedMhz} MT/s.",
                $"✅ EXPO / XMP active: DDR-{ram.ConfiguredSpeedMhz} MT/s."));
        }

        if (bios.IsOutdated)
        {
            score -= 15;
            items.Add(LocalizationService.Pick(
                "⚠️ Das Mainboard-BIOS ist älter als 9 Monate. Ein AGESA-Update bringt in der Regel spürbar mehr PBO-Stabilität.",
                "⚠️ The motherboard BIOS is more than 9 months old. An AGESA update usually brings noticeably better PBO stability."));
        }
        else
        {
            items.Add(LocalizationService.Pick(
                $"✅ BIOS aktuell: {bios.BiosVersion} ({bios.BiosVendor}).",
                $"✅ BIOS current: {bios.BiosVersion} ({bios.BiosVendor})."));
        }

        return new SystemHealthAudit(ram, bios, Math.Max(0, score), items);
    }

    public static RamAuditResult AuditRam()
    {
        int configuredSpeed = 0;
        int maxSpeed = 0;

        try
        {
            using var searcher = new ManagementObjectSearcher("SELECT Speed, ConfiguredClockSpeed FROM Win32_PhysicalMemory");
            foreach (ManagementObject obj in searcher.Get())
            {
                if (obj["ConfiguredClockSpeed"] is uint cSpeed && cSpeed > 0)
                    configuredSpeed = Math.Max(configuredSpeed, (int)cSpeed);
                else if (obj["ConfiguredClockSpeed"] is int cSpeedInt && cSpeedInt > 0)
                    configuredSpeed = Math.Max(configuredSpeed, cSpeedInt);

                if (obj["Speed"] is uint speed && speed > 0)
                    maxSpeed = Math.Max(maxSpeed, (int)speed);
                else if (obj["Speed"] is int speedInt && speedInt > 0)
                    maxSpeed = Math.Max(maxSpeed, speedInt);
            }
        }
        catch
        {
            // Fallback for non-WMI or restricted environments
            configuredSpeed = 4800;
            maxSpeed = 6000;
        }

        if (configuredSpeed == 0) configuredSpeed = 4800;
        if (maxSpeed == 0) maxSpeed = configuredSpeed;

        // Standard JEDEC fallbacks for DDR4/DDR5: <= 2666, 3200, 4800
        bool isXmpActive = configuredSpeed > 3200 && configuredSpeed >= (maxSpeed - 200);
        string advice = isXmpActive
            ? LocalizationService.Pick($"EXPO / XMP ist aktiv ({configuredSpeed} MHz).", $"EXPO / XMP is active ({configuredSpeed} MHz).")
            : LocalizationService.Pick(
                $"EXPO / XMP ist deaktiviert: RAM läuft mit {configuredSpeed} MHz statt möglicher {maxSpeed} MHz. Im BIOS EXPO/DOCP einschalten.",
                $"EXPO / XMP is disabled: RAM runs at {configuredSpeed} MHz instead of a possible {maxSpeed} MHz. Enable EXPO/DOCP in the BIOS.");

        return new RamAuditResult(configuredSpeed, maxSpeed, isXmpActive, advice);
    }

    public static BiosAuditResult AuditBios()
    {
        string vendor = "Unknown";
        string version = "Unknown";
        DateTime? releaseDate = null;
        bool isOutdated = false;

        try
        {
            using var searcher = new ManagementObjectSearcher("SELECT Manufacturer, SMBIOSBIOSVersion, ReleaseDate FROM Win32_BIOS");
            foreach (ManagementObject obj in searcher.Get())
            {
                vendor = obj["Manufacturer"]?.ToString() ?? "Unknown";
                version = obj["SMBIOSBIOSVersion"]?.ToString() ?? "Unknown";

                if (obj["ReleaseDate"] is string dateStr && dateStr.Length >= 8)
                {
                    string yearStr = dateStr.Substring(0, 4);
                    string monthStr = dateStr.Substring(4, 2);
                    string dayStr = dateStr.Substring(6, 2);
                    if (DateTime.TryParse($"{yearStr}-{monthStr}-{dayStr}", out DateTime dt))
                    {
                        releaseDate = dt;
                        if ((DateTime.Now - dt).TotalDays > 270) // Older than 9 months
                        {
                            isOutdated = true;
                        }
                    }
                }
            }
        }
        catch { }

        string advice = isOutdated
            ? LocalizationService.Pick(
                $"BIOS {version} ({releaseDate:MMM yyyy}) ist über 9 Monate alt. Ein Update bringt meist bessere Ryzen-Stabilität.",
                $"BIOS {version} ({releaseDate:MMM yyyy}) is over 9 months old. An update usually improves Ryzen stability.")
            : LocalizationService.Pick($"BIOS {version} ist auf aktuellem Stand.", $"BIOS {version} is up to date.");

        return new BiosAuditResult(vendor, version, releaseDate, isOutdated, advice);
    }
}
