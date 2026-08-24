using System.IO.Compression;
using System.Text.RegularExpressions;

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
/// Where an engine comes from. Several mirrors rather than one address, because these are
/// version-pinned files on servers that prune old releases: the original single URLs for both
/// engines had gone 404, which turned first-run setup into a dead end.
/// </summary>
/// <param name="Urls">
/// Pinned, known-good builds. Tried first so the version stays predictable.
/// </param>
/// <param name="PageUrl">
/// The vendor's download page. Serves two purposes: the current archive link is discovered
/// from it when every pinned URL has gone, and it is offered to the user for a manual install
/// if even that fails.
/// </param>
/// <param name="LinkPattern">
/// Matches an archive link on <paramref name="PageUrl"/>. This is what makes the setup
/// self-healing: both vendors prune old releases, so a hard-coded version is guaranteed to
/// 404 eventually - as both of them already had.
/// </param>
public sealed record EngineSource(
    string Name,
    IReadOnlyList<string> Urls,
    string PageUrl,
    string LinkPattern,
    string TargetSubdirectory
);

/// <summary>
/// Verifies external stress engines (Prime95, y-cruncher) and kernel driver (PawnIO),
/// providing automated downloads for missing test engines.
/// </summary>
public static class DependencyService
{
    private static readonly HttpClient HttpClient = new() { Timeout = TimeSpan.FromMinutes(10) };

    /// <summary>
    /// 30.8 build 17 deliberately, not the newest: it is the build this app's prime.txt
    /// generation and results.txt parsing were written against, and the one CoreCycler
    /// standardised on.
    /// </summary>
    public static readonly EngineSource Prime95 = new(
        "Prime95",
        [
            "https://www.mersenne.org/download/software/v30/30.8/p95v308b17.win64.zip",
            "https://download.mersenne.ca/gimps/v30/30.8/p95v308b17.win64.zip",
        ],
        "https://www.mersenne.org/download/",
        @"https?://[^""'\s<>]*p95v[0-9b.]+\.win64\.zip",
        "prime95");

    public static readonly EngineSource YCruncher = new(
        "y-cruncher",
        [
            "https://cdn.numberworld.org/y-cruncher-downloads/y-cruncher%20v0.8.7.9547b.zip",
            "http://www.numberworld.org/y-cruncher/y-cruncher%20v0.8.7.9547b.zip",
        ],
        "http://www.numberworld.org/y-cruncher/",
        @"https?://[^""'\s<>]*y-cruncher[^""'\s<>]*\.zip",
        "ycruncher");

    public const string PawnIoDownloadUrl = "https://github.com/namazso/PawnIO.Setup/releases/download/2.2.0/PawnIO_setup.exe";
    public const string PawnIoPageUrl = "https://pawnio.eu/";

    /// <summary>
    /// Where the offline bundle puts the driver installer. Checked before reaching for the
    /// network, so a machine with no internet can still install the driver - without which the
    /// whole point of the offline bundle is lost, since Curve Optimizer access needs it.
    /// </summary>
    public static string BundledPawnIoPath(string baseDirectory) =>
        Path.Combine(baseDirectory, "drivers", "PawnIO_setup.exe");

    public static bool HasBundledPawnIo(string baseDirectory) =>
        File.Exists(BundledPawnIoPath(baseDirectory));

    public static string TargetDirectoryFor(string baseDirectory, EngineSource source) =>
        Path.Combine(baseDirectory, "engines", source.TargetSubdirectory);

    public static async Task DownloadAndInstallPawnIoAsync(
        string baseDirectory, Action<string>? progress = null, CancellationToken ct = default)
    {
        string installer;

        if (HasBundledPawnIo(baseDirectory))
        {
            installer = BundledPawnIoPath(baseDirectory);
            progress?.Invoke(LocalizationService.Pick(
                "Mitgelieferter PawnIO-Installer wird verwendet…",
                "Using the bundled PawnIO installer…"));
        }
        else
        {
            installer = Path.Combine(Path.GetTempPath(), "PawnIO_setup.exe");
            progress?.Invoke(LocalizationService.Pick("PawnIO-Installer wird heruntergeladen…", "Downloading the PawnIO installer…"));

            using var response = await HttpClient.GetAsync(PawnIoDownloadUrl, HttpCompletionOption.ResponseHeadersRead, ct);
            response.EnsureSuccessStatusCode();
            using var fileStream = new FileStream(installer, FileMode.Create, FileAccess.Write, FileShare.None, 8192, true);
            await response.Content.CopyToAsync(fileStream, ct);
        }

        progress?.Invoke(LocalizationService.Pick("PawnIO-Installer wird gestartet (Administratorrechte erforderlich)…", "Starting the PawnIO installer (administrator rights required)…"));
        var psi = new System.Diagnostics.ProcessStartInfo(installer)
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

    /// <summary>
    /// Thrown when no mirror worked. Carries the vendor page so the caller can offer it
    /// instead of showing a bare HTTP status.
    /// </summary>
    public sealed class EngineDownloadException(string message, EngineSource source) : Exception(message)
    {
        public EngineSource Source { get; } = source;
    }

    /// <summary>
    /// Pinned builds first, then whatever the vendor page currently offers, and only then give
    /// up. The middle step is the one that keeps this working: a version-pinned URL is fine
    /// until the vendor deletes that release, and then it is a dead end forever.
    /// </summary>
    public static async Task DownloadEngineAsync(
        string baseDirectory, EngineSource source,
        Action<double, string>? progress = null, CancellationToken ct = default)
    {
        string targetDir = TargetDirectoryFor(baseDirectory, source);
        var failures = new List<string>();
        var tried = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

        async Task<bool> TryUrlAsync(string url, bool discovered)
        {
            if (!tried.Add(url)) return false;
            try
            {
                if (discovered)
                {
                    progress?.Invoke(0, LocalizationService.Pick(
                        $"Aktuelle Version gefunden: {FileNameOf(url)}",
                        $"Found current version: {FileNameOf(url)}"));
                }
                await DownloadAndExtractAsync(url, targetDir, progress, ct);
                return true;
            }
            catch (OperationCanceledException)
            {
                throw;
            }
            catch (Exception ex)
            {
                failures.Add($"{new Uri(url).Host}: {ex.Message}");
                return false;
            }
        }

        foreach (string url in source.Urls)
        {
            ct.ThrowIfCancellationRequested();
            if (await TryUrlAsync(url, discovered: false)) return;
        }

        progress?.Invoke(-1, LocalizationService.Pick(
            "Feste Download-Adresse nicht mehr gültig — aktuelle Version wird gesucht…",
            "The pinned download address is gone — looking up the current version…"));

        foreach (string url in await DiscoverUrlsAsync(source, ct))
        {
            ct.ThrowIfCancellationRequested();
            if (await TryUrlAsync(url, discovered: true)) return;
        }

        throw new EngineDownloadException(
            LocalizationService.Pick(
                $"{source.Name} konnte von keiner Quelle geladen werden ({string.Join("; ", failures)}).",
                $"{source.Name} could not be downloaded from any source ({string.Join("; ", failures)})."),
            source);
    }

    private static string FileNameOf(string url)
    {
        try { return Uri.UnescapeDataString(Path.GetFileName(new Uri(url).AbsolutePath)); }
        catch { return url; }
    }

    /// <summary>
    /// Reads the vendor's download page and pulls out archive links. Best-effort by nature:
    /// a page redesign breaks it, which is why it runs after the pinned URLs and why failure
    /// still ends with a link the user can click.
    /// </summary>
    private static async Task<IReadOnlyList<string>> DiscoverUrlsAsync(EngineSource source, CancellationToken ct)
    {
        try
        {
            string html = await HttpClient.GetStringAsync(source.PageUrl, ct);

            return [.. Regex.Matches(html, source.LinkPattern, RegexOptions.IgnoreCase)
                        .Select(m => m.Value)
                        .Distinct(StringComparer.OrdinalIgnoreCase)
                        .Take(4)];
        }
        catch (OperationCanceledException)
        {
            throw;
        }
        catch
        {
            return [];
        }
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

            if (!response.IsSuccessStatusCode)
            {
                throw new HttpRequestException(LocalizationService.Pick(
                    $"HTTP {(int)response.StatusCode}",
                    $"HTTP {(int)response.StatusCode}"));
            }

            long totalBytes = response.Content.Headers.ContentLength ?? -1L;
            using (var contentStream = await response.Content.ReadAsStreamAsync(ct))
            using (var fileStream = new FileStream(tempZip, FileMode.Create, FileAccess.Write, FileShare.None, 8192, true))
            {
                var buffer = new byte[16384];
                long totalRead = 0;
                int read;

                while ((read = await contentStream.ReadAsync(buffer, ct)) > 0)
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
            }

            // A server that answers a dead link with an HTML page would otherwise surface as an
            // opaque "not a zip archive" failure from deep inside ZipFile.
            if (!LooksLikeZip(tempZip))
            {
                throw new InvalidDataException(LocalizationService.Pick(
                    "Die Antwort war kein ZIP-Archiv (vermutlich eine Fehlerseite).",
                    "The response was not a ZIP archive (most likely an error page)."));
            }

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

    private static bool LooksLikeZip(string path)
    {
        try
        {
            using var fs = File.OpenRead(path);
            return fs.Length > 4 && fs.ReadByte() == 'P' && fs.ReadByte() == 'K';
        }
        catch
        {
            return false;
        }
    }
}
