using System.Diagnostics;
using System.Text;

namespace PboStudio.Core;

/// <summary>Coarse CPU generation, only as fine-grained as engine binary selection needs.</summary>
public enum ZenGeneration { Unknown, Zen1, Zen2, Zen3, Zen4, Zen5 }

public sealed record YCruncherOptions(
    IReadOnlyList<string> Algorithms,
    int SecondsPerTest = 60,
    string Memory = "64M",
    bool SpreadAcrossSmt = false)
{
    /// <summary>
    /// Every algorithm tag y-cruncher 0.8.x accepts, in the order its own menu lists them.
    /// <para>
    /// The names matter literally: they are passed straight to the command line, and the
    /// v0.7 spellings "SFT" and "FFT" were silently renamed to "SFTv4" and "FFTv4". They are
    /// not aliases - "Command Lines.txt" lists aliases explicitly (N64, VST) and these two are
    /// not among them - so the old tags asked for tests that no longer exist under that name.
    /// </para>
    /// </summary>
    public static readonly IReadOnlyList<string> AllAlgorithms =
        ["BKT", "BBP", "SFTv4", "SNT", "SVT", "FFTv4", "N63", "VT3"];

    /// <summary>
    /// VT3 earns its place in the default set: it is the algorithm most often credited with
    /// catching Curve Optimizer instability that Prime95 runs straight past.
    /// </summary>
    public static YCruncherOptions Default => new(AllAlgorithms);

    public static YCruncherOptions CurveOptimizer => new(["SFTv4", "SVT", "FFTv4", "N63", "VT3"]);

    public static YCruncherOptions FastDiscovery => new(["VT3", "FFTv4", "N63"]);
}

/// <summary>
/// Drives y-cruncher's component stress tester, the second opinion to Prime95.
///
/// It is worth having precisely because it fails differently: its algorithms lean on the
/// integer and memory paths in ways Prime95's FFT work does not, so a core that passes hours
/// of Prime95 can still fall over here.
/// </summary>
public sealed class YCruncherEngine : IStressEngine
{
    private readonly string _exePath;
    private readonly string _workRoot;
    private readonly YCruncherOptions _options;

    public YCruncherEngine(string exePath, string workRoot, YCruncherOptions? options = null)
    {
        _exePath = exePath;
        _workRoot = workRoot;
        _options = options ?? YCruncherOptions.CurveOptimizer;
    }

    /// <summary>
    /// The binaries worth offering by hand, coldest instruction set first. Which one runs
    /// changes what is actually being tested: "00-x86" produces the least heat and therefore
    /// the highest boost clock, while the Zen-native builds exercise AVX2/AVX-512 and pull far
    /// more current. A Curve Optimizer value that holds under one can fail under the other, so
    /// picking the binary is a test decision, not an installation detail.
    /// </summary>
    public static readonly IReadOnlyList<(string Prefix, string Label)> SelectableBinaries =
    [
        ("00-x86", "00-x86 · Legacy x86 (lowest load, highest boost)"),
        ("04-P4P", "04-P4P · SSE3"),
        ("13-HSW", "13-HSW ~ Airi · AVX2"),
        ("17-ZN1", "17-ZN1 ~ Yukina · Zen 1"),
        ("19-ZN2", "19-ZN2 ~ Kagari · Zen 2/3 (AVX2)"),
        ("22-ZN4", "22-ZN4 ~ Kizuna · Zen 4 (AVX-512)"),
        ("24-ZN5", "24-ZN5 ~ Komari · Zen 5 (AVX-512)"),
    ];

    /// <summary>Binaries this installation actually ships, so the picker cannot offer a dead entry.</summary>
    public static IReadOnlyList<(string Prefix, string Label)> AvailableBinaries(string installRoot)
    {
        string binaries = Path.Combine(installRoot, "Binaries");
        if (!Directory.Exists(binaries)) return [];

        var present = Directory.EnumerateFiles(binaries, "*.exe").Select(Path.GetFileName).ToList();
        return [.. SelectableBinaries.Where(b =>
            present.Any(f => f!.StartsWith(b.Prefix, StringComparison.OrdinalIgnoreCase)))];
    }

    /// <summary>
    /// Picks the binary matching this CPU from y-cruncher's Binaries folder.
    ///
    /// The top-level y-cruncher.exe is only a launcher: it selects one of these and runs it as a
    /// child process, which leaves nothing to pin - the parent does no work and the child was
    /// never ours to configure. Addressing the binary directly is what makes single-core testing
    /// possible at all.
    /// </summary>
    /// <param name="explicitPrefix">
    /// Overrides the automatic choice. Falls back to auto-detection when the requested binary
    /// is not present, because an installation that lacks it must still be able to run.
    /// </param>
    public static string? FindBinaryFor(string installRoot, ZenGeneration generation, string? explicitPrefix = null)
    {
        string binaries = Path.Combine(installRoot, "Binaries");
        if (!Directory.Exists(binaries)) return null;

        if (!string.IsNullOrWhiteSpace(explicitPrefix))
        {
            string? chosen = Directory
                .EnumerateFiles(binaries, "*.exe")
                .FirstOrDefault(f => Path.GetFileName(f).StartsWith(explicitPrefix, StringComparison.OrdinalIgnoreCase));
            if (chosen is not null) return chosen;
        }

        // Names carry their architecture as a prefix, e.g. "19-ZN2 ~ Kagari.exe".
        string[] preferred = generation switch
        {
            ZenGeneration.Zen5 => ["24-ZN5", "22-ZN4", "19-ZN2"],
            ZenGeneration.Zen4 => ["22-ZN4", "19-ZN2"],
            ZenGeneration.Zen3 or ZenGeneration.Zen2 => ["19-ZN2", "17-ZN1"],
            ZenGeneration.Zen1 => ["17-ZN1", "13-HSW"],
            _ => ["13-HSW", "11-SNB", "00-x86"],
        };

        foreach (string prefix in preferred)
        {
            string? hit = Directory
                .EnumerateFiles(binaries, "*.exe")
                .FirstOrDefault(f => Path.GetFileName(f).StartsWith(prefix, StringComparison.OrdinalIgnoreCase));
            if (hit is not null) return hit;
        }

        return Directory.EnumerateFiles(binaries, "*.exe").FirstOrDefault();
    }

    public EngineKind Kind => EngineKind.YCruncher;
    public string DisplayName => "y-cruncher";
    public bool IsAvailable => File.Exists(_exePath);

    public IStressSession Start(PhysicalCore core, int threads)
    {
        if (!IsAvailable)
            throw new FileNotFoundException("y-cruncher is not installed.", _exePath);

        string workDir = Path.Combine(_workRoot, $"yc-core{core.Index}");
        Directory.CreateDirectory(workDir);

        // A previous run may still be releasing the file; a fresh name is simpler than waiting.
        string logPath = Path.Combine(workDir, $"output-{DateTime.Now:HHmmssfff}.txt");

        // Order matters: logfile is a global switch and has to precede the "stress" verb.
        string arguments =
            $"logfile:\"{logPath}\" stress -M:{_options.Memory} -D:{_options.SecondsPerTest} " +
            string.Join(' ', _options.Algorithms);

        nuint mask = core.MaskFor(threads, _options.SpreadAcrossSmt);

        // The jail has to exist before the process runs any code, otherwise y-cruncher has
        // already read the machine's full processor count and sized itself for it.
        var jail = new CoreJail(mask);
        var process = PinnedProcess.Start(
            _exePath, arguments, Path.GetDirectoryName(_exePath)!, mask, jail);

        return new YCruncherSession(process, logPath, jail);
    }
}

internal sealed class YCruncherSession : IStressSession
{
    // Phrases that only ever appear on an actual failure. Matching the bare word "error" turned
    // y-cruncher's own settings banner ("Stop on Error: Enabled") into a reported defect.
    private static readonly string[] ErrorMarkers =
    [
        "Coefficient is too large",
        "computation error",
        "hardware error",
        "mismatch",
        "test failed",
        "FAILED",
        "incorrect result",
    ];

    private readonly Process _process;
    private readonly string _logPath;
    private readonly CoreJail? _jail;
    private readonly List<Failure> _pending = [];
    private long _offset;
    private bool _reportedExit;

    public YCruncherSession(Process process, string logPath, CoreJail? jail = null)
    {
        _process = process;
        _logPath = logPath;
        _jail = jail;
    }

    public bool IsAlive
    {
        get { try { return !_process.HasExited; } catch { return false; } }
    }

    public TimeSpan CpuTime
    {
        get { try { _process.Refresh(); return _process.TotalProcessorTime; } catch { return TimeSpan.Zero; } }
    }

    public IReadOnlyList<Failure> DrainFailures()
    {
        ScanLog();

        // y-cruncher stops of its own accord when a computation goes wrong, so an unexpected
        // exit is itself the signal - the log may not even have been flushed yet.
        if (!IsAlive && !_reportedExit)
        {
            _reportedExit = true;
            _pending.Add(new Failure(FailureKind.ProcessGone,
                "y-cruncher beendete sich selbst - das tut es bei einem Rechenfehler.",
                DateTime.Now));
        }

        if (_pending.Count == 0) return [];
        var drained = _pending.ToArray();
        _pending.Clear();
        return drained;
    }

    /// <summary>
    /// y-cruncher prints "Iteration: N" once per complete pass over the selected algorithms.
    /// The header for pass N appears when that pass begins, so the count of headers seen minus
    /// the one currently running is the number of finished sweeps.
    /// </summary>
    private int _iterationHeaders;

    public int WorkloadCyclesCompleted => Math.Max(0, _iterationHeaders - 1);

    private void ScanLog()
    {
        if (!File.Exists(_logPath)) return;

        try
        {
            using var stream = new FileStream(_logPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite);
            if (stream.Length <= _offset) return;

            stream.Seek(_offset, SeekOrigin.Begin);
            using var reader = new StreamReader(stream);

            while (reader.ReadLine() is { } line)
            {
                if (line.Contains("Iteration:", StringComparison.OrdinalIgnoreCase)) _iterationHeaders++;

                if (line.Contains("passed", StringComparison.OrdinalIgnoreCase)) continue;

                // Expected noise: the job object denies every processor outside the mask, and
                // y-cruncher announces each refusal. That is the confinement working, not a fault.
                if (line.Contains("core affinity", StringComparison.OrdinalIgnoreCase)) continue;

                if (ErrorMarkers.Any(m => line.Contains(m, StringComparison.OrdinalIgnoreCase)))
                    _pending.Add(new Failure(FailureKind.CalculationError, line.Trim(), DateTime.Now));
            }

            _offset = stream.Length;
        }
        catch (IOException) { }
    }

    public void Pause(TimeSpan duration)
    {
        if (!IsAlive) return;

        try
        {
            Native.NtSuspendProcess(_process.Handle);
            Thread.Sleep(duration);
        }
        catch { }
        finally
        {
            try { Native.NtResumeProcess(_process.Handle); } catch { }
        }
    }

    public void Stop()
    {
        try { if (!_process.HasExited) _process.Kill(entireProcessTree: true); }
        catch { }
    }

    public void Dispose()
    {
        Stop();
        _process.Dispose();

        // Closing the job kills anything still inside it, so no worker survives the session.
        _jail?.Dispose();
    }
}
