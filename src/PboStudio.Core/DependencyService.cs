using System.IO.Compression;

namespace PboStudio.Core;

public sealed record EngineStatus(
    bool Prime95Available,
    string Prime95Path,
    bool YCruncherAvailable,
    string? YCruncherPath,
    bool PawnIoAvailable,
    string PawnIoStatus
);

/// <summary>
/// Verifies external stress engines (Prime95, y-cruncher) and kernel driver (PawnIO),
/// providing automated downloads for missing test engines.
/// </summary>
public static class DependencyService
{
    private static readonly HttpClient HttpClient = new();

    // Standard download links for official release archives
    public const string Prime95DownloadUrl = "https://www.mersenne.ca/gimps/p95v308b17.win64.zip";
    public const string YCruncherDownloadUrl = "http://www.numberworld.org/y-cruncher/y-cruncher%20v0.8.5.9542.zip";
    public const string PawnIoDownloadUrl = "https://github.com/namazso/PawnIO.Setup/releases/download/2.2.0/PawnIO_setup.exe";

    public static async Task DownloadAndInstallPawnIoAsync(Action<string>? progress = null, CancellationToken ct = default)
    {
        string tempExe = Path.Combine(Path.GetTempPath(), "PawnIO_setup.exe");
        progress?.Invoke(LocalizationService.Pick("PawnIO-Installer wird heruntergeladen…", "Downloading the PawnIO installer…"));

        using (var response = await HttpClient.GetAsync(PawnIoDownloadUrl, HttpCompletionOption.ResponseHeadersRead, ct))
        {
            response.EnsureSuccessStatusCode();
            using var fileStream = new FileStream(tempExe, FileMode.Create, FileAccess.Write, FileShare.None, 8192, true);
            await response.Content.CopyToAsync(fileStream, ct);
        }

        progress?.Invoke(LocalizationService.Pick("PawnIO-Installer wird gestartet (Administratorrechte erforderlich)…", "Starting the PawnIO installer (administrator rights required)…"));
        var psi = new System.Diagnostics.ProcessStartInfo(tempExe)
        {
            UseShellExecute = true,
            Verb = "runas"
        };
        System.Diagnostics.Process.Start(psi);
    }

    public static EngineStatus CheckDependencies(string baseDirectory, SmuService smu)
    {
        string engineRoot = Path.Combine(baseDirectory, "engines");

        string p95Path = Path.Combine(engineRoot, "prime95", "prime95.exe");
        bool p95Ok = File.Exists(p95Path);

        string ycRoot = Path.Combine(engineRoot, "ycruncher");
        string? ycPath = null;
        if (Directory.Exists(ycRoot))
        {
            string? installDir = Directory.EnumerateDirectories(ycRoot, "y-cruncher*").FirstOrDefault();
            if (installDir is not null)
                ycPath = YCruncherEngine.FindBinaryFor(installDir, ZenGeneration.Zen4);
        }
        bool ycOk = ycPath is not null && File.Exists(ycPath);

        bool pawnIoOk = smu.IsAvailable;
        string pawnIoText = pawnIoOk
            ? LocalizationService.Pick("PawnIO-Treiber geladen und SMU erreichbar.", "PawnIO driver loaded and SMU reachable.")
            : smu.InitError ?? LocalizationService.Pick(
                "PawnIO fehlt oder Administratorrechte fehlen.",
                "PawnIO is missing or administrator rights are missing.");

        return new EngineStatus(p95Ok, p95Path, ycOk, ycPath, pawnIoOk, pawnIoText);
    }

    public static async Task DownloadAndExtractAsync(
        string url, string targetDir, Action<double, string>? progress = null, CancellationToken ct = default)
    {
        Directory.CreateDirectory(targetDir);
        string tempZip = Path.Combine(Path.GetTempPath(), $"pbo_download_{Guid.NewGuid():N}.zip");

        try
        {
            progress?.Invoke(0, LocalizationService.Pick("Verbindung wird aufgebaut…", "Connecting…"));
            using var response = await HttpClient.GetAsync(url, HttpCompletionOption.ResponseHeadersRead, ct);
            response.EnsureSuccessStatusCode();

            long totalBytes = response.Content.Headers.ContentLength ?? -1L;
            using var contentStream = await response.Content.ReadAsStreamAsync(ct);
            using var fileStream = new FileStream(tempZip, FileMode.Create, FileAccess.Write, FileShare.None, 8192, true);

            var buffer = new byte[16384];
            long totalRead = 0;
            int read;

            while ((read = await contentStream.ReadAsync(buffer, 0, buffer.Length, ct)) > 0)
            {
                await fileStream.WriteAsync(buffer.AsMemory(0, read), ct);
                totalRead += read;
                if (totalBytes > 0)
                {
                    double pct = (double)totalRead / totalBytes * 100.0;
                    progress?.Invoke(pct, LocalizationService.Pick(
                        $"Geladen: {totalRead / 1024 / 1024} MB von {totalBytes / 1024 / 1024} MB ({pct:F0} %)",
                        $"Downloaded: {totalRead / 1024 / 1024} MB of {totalBytes / 1024 / 1024} MB ({pct:F0} %)"));
                }
                else
                {
                    progress?.Invoke(-1, LocalizationService.Pick($"Geladen: {totalRead / 1024 / 1024} MB", $"Downloaded: {totalRead / 1024 / 1024} MB"));
                }
            }

            await fileStream.DisposeAsync();

            progress?.Invoke(100, LocalizationService.Pick("Wird entpackt…", "Extracting…"));
            ZipFile.ExtractToDirectory(tempZip, targetDir, overwriteFiles: true);
            progress?.Invoke(100, LocalizationService.Pick("Fertig.", "Done."));
        }
        finally
        {
            if (File.Exists(tempZip))
            {
                try { File.Delete(tempZip); } catch { }
            }
        }
    }
}
