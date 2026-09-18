using System.Text.RegularExpressions;
using System.Diagnostics;
using System.Text;

namespace PboStudio.Core;

public enum Prime95Mode { Sse, Avx, Avx2, Avx512 }

/// <summary>FFT ranges in K. Small sizes stay in cache and load the core hardest, which is
/// what exposes an undervolted core; large sizes pull in the memory controller instead.</summary>
/// <summary>
/// FFT ranges in K, matching CoreCycler's presets so results are comparable between the two.
/// Heavy and HeavyShort are the ones most people reach for on Curve Optimizer work: they sweep
/// the in-cache sizes that expose an undervolted core fastest.
/// </summary>
public enum FftPreset { Smallest, Small, Large, Huge, All, Heavy, HeavyShort, Moderate, Custom }

public sealed record Prime95Options(
    Prime95Mode Mode = Prime95Mode.Avx2,
    FftPreset Fft = FftPreset.Smallest,
    int CustomMinFft = 4,
    int CustomMaxFft = 32,
    /// <summary>
    /// Prime95's <c>TortureMem</c>, in MB. Zero keeps the whole test in cache, which is what
    /// exposes an undervolted core; anything above it pulls the memory controller in as well
    /// and tests a different thing.
    /// </summary>
    int MemoryMb = 0,
    bool SpreadAcrossSmt = false,
    ProcessPriorityClass Priority = ProcessPriorityClass.Normal)
{
    public (int Min, int Max) FftRange => Fft switch
    {
        FftPreset.Smallest => (4, 21),
        FftPreset.Small => (36, 248),
        FftPreset.Large => (426, 8192),
        FftPreset.Huge => (8960, 32768),
        FftPreset.All => (4, 32768),
        FftPreset.Heavy => (4, 1344),
        FftPreset.HeavyShort => (4, 160),
        FftPreset.Moderate => (1344, 4096),
        _ => (CustomMinFft, CustomMaxFft),
    };

    /// <summary>Short label for the UI, e.g. "Heavy (4-1344K)".</summary>
    public string FftLabel
    {
        get
        {
            var (min, max) = FftRange;
            return $"{Fft} ({min}-{max}K)";
        }
    }
}

/// <summary>
/// Drives Prime95 in torture-test mode.
///
/// Two behaviours of Prime95 30.x shaped this: it folds local.txt into prime.txt on start,
/// so every setting goes into the one file; and it picks its own worker count unless
/// NumWorkers and CoresPerTest are pinned to 1, which otherwise spreads load across cores
/// and makes single-core testing meaningless.
/// </summary>
public sealed class Prime95Engine : IStressEngine
{
    private readonly string _exePath;
    private readonly string _workRoot;
    private readonly Prime95Options _options;

    public Prime95Engine(string exePath, string workRoot, Prime95Options? options = null)
    {
        _exePath = exePath;
        _workRoot = workRoot;
        _options = options ?? new Prime95Options();
    }

    public EngineKind Kind => EngineKind.Prime95;
    public string DisplayName => "Prime95";
    public bool IsAvailable => File.Exists(_exePath);

    public IStressSession Start(PhysicalCore core, int threads)
    {
        if (!IsAvailable)
            throw new FileNotFoundException("Prime95 is not installed.", _exePath);

        // A directory per core keeps each worker's results.txt separate, so a failure can be
        // attributed without parsing which worker wrote which line.
        string workDir = Path.Combine(_workRoot, $"core{core.Index}");
        if (Directory.Exists(workDir))
        {
            try
            {
                foreach (var file in Directory.EnumerateFiles(workDir))
                    File.Delete(file);
            }
            catch { /* best effort cleanup */ }
        }
        Directory.CreateDirectory(workDir);

        string resultsPath = Path.Combine(workDir, "results.txt");
        if (File.Exists(resultsPath)) File.Delete(resultsPath);

        // Delete stale local.txt to avoid overriding prime.txt settings
        string localTxt = Path.Combine(workDir, "local.txt");
        if (File.Exists(localTxt)) File.Delete(localTxt);

        File.WriteAllText(Path.Combine(workDir, "prime.txt"), BuildConfig(threads), Encoding.ASCII);

        nuint mask = core.MaskFor(threads, _options.SpreadAcrossSmt);

        // Use CoreJail and PinnedProcess so Prime95 worker threads are strictly confined
        // to their assigned affinity mask from the very first instruction.
        var jail = new CoreJail(mask);
        string arguments = $"-W\"{workDir}\" -t";
        var process = PinnedProcess.Start(_exePath, arguments, workDir, mask, jail, _options.Priority);

        return new Prime95Session(process, resultsPath, jail);
    }

    private string BuildConfig(int threads)
    {
        var (minFft, maxFft) = _options.FftRange;
        var sb = new StringBuilder();

        sb.AppendLine("StressTester=1");
        sb.AppendLine("UsePrimenet=0");
        sb.AppendLine("V24OptionsConverted=1");
        sb.AppendLine("WGUID_version=2");

        // Round-off and input checking is what turns a wrong result into a reported error
        // rather than a silently corrupted one.
        sb.AppendLine("ErrorCheck=1");
        sb.AppendLine("SumInputsErrorCheck=1");

        sb.AppendLine($"CpuSupportsAVX={(_options.Mode >= Prime95Mode.Avx ? 1 : 0)}");
        sb.AppendLine($"CpuSupportsAVX2={(_options.Mode >= Prime95Mode.Avx2 ? 1 : 0)}");
        sb.AppendLine($"CpuSupportsFMA3={(_options.Mode >= Prime95Mode.Avx2 ? 1 : 0)}");
        sb.AppendLine($"CpuSupportsAVX512={(_options.Mode >= Prime95Mode.Avx512 ? 1 : 0)}");

        sb.AppendLine($"MinTortureFFT={minFft}");
        sb.AppendLine($"MaxTortureFFT={maxFft}");
        sb.AppendLine($"TortureMem={_options.MemoryMb}");
        sb.AppendLine("TortureTime=1");
        sb.AppendLine($"TortureThreads={threads}");
        sb.AppendLine("TortureHyperthreading=0");

        // Without these Prime95 chooses its own layout and spreads across cores.
        sb.AppendLine("NumWorkers=1");
        sb.AppendLine("CoresPerTest=1");

        return sb.ToString();
    }
}

internal sealed class Prime95Session : IStressSession
{
    // Prime95 writes its successes and its failures into the same file, and the word "self-test"
    // appears in both ("Self-test 8960K passed!" / "...self-test failed"). Matching on the bare
    // word turned every completed FFT size into a false alarm, so the markers must be phrases
    // that only ever occur on failure, and any line announcing a pass is excluded outright.
    private static readonly string[] ErrorMarkers =
    [
        "FATAL ERROR",
        "Hardware failure detected",
        "self-test failed",
        "ERROR: ROUND OFF",
        "Rounding was",
        "Illegal sumout",
        "SUMOUT",
        "TORTURE TEST FAILED",
    ];

    private readonly Process _process;
    private readonly string _resultsPath;
    private readonly CoreJail? _jail;
    private readonly List<Failure> _pending = [];
    private long _resultsOffset;
    private bool _reportedExit;

    public Prime95Session(Process process, string resultsPath, CoreJail? jail = null)
    {
        _process = process;
        _resultsPath = resultsPath;
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
        ScanResults();

        if (!IsAlive && !_reportedExit)
        {
            _reportedExit = true;
            _pending.Add(new Failure(FailureKind.ProcessGone,
                "Prime95 exited on its own - it does not do that during a torture test.",
                DateTime.Now));
        }

        if (_pending.Count == 0) return [];
        var drained = _pending.ToArray();
        _pending.Clear();
        return drained;
    }

    /// <summary>
    /// Pulls the FFT length out of a Prime95 pass line, e.g.
    /// "Self-test 8960K passed!" or "Self-test 1024K passed!".
    /// </summary>
    private static readonly Regex PassedFft =
        new(@"self-test\s+(\d+)K\s+passed", RegexOptions.IgnoreCase | RegexOptions.Compiled);

    /// <summary>
    /// One full sweep is done the moment an FFT size comes round a second time: Prime95 walks
    /// the preset in order and then starts over, so a repeat is the only signal it gives that
    /// the list has been exhausted.
    /// </summary>
    private readonly HashSet<int> _seenFftSizes = [];
    private int _cycles;

    public int WorkloadCyclesCompleted => _cycles;

    private void ScanResults()
    {
        if (!File.Exists(_resultsPath)) return;

        try
        {
            using var stream = new FileStream(_resultsPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite);
            if (stream.Length <= _resultsOffset) return;

            stream.Seek(_resultsOffset, SeekOrigin.Begin);
            using var reader = new StreamReader(stream, Encoding.ASCII);

            while (reader.ReadLine() is { } line)
            {
                if (line.Contains("passed", StringComparison.OrdinalIgnoreCase))
                {
                    if (PassedFft.Match(line) is { Success: true } m
                        && int.TryParse(m.Groups[1].Value, out int fft)
                        && !_seenFftSizes.Add(fft))
                    {
                        _cycles++;
                        _seenFftSizes.Clear();
                        _seenFftSizes.Add(fft);
                    }
                    continue;
                }

                if (ErrorMarkers.Any(m => line.Contains(m, StringComparison.OrdinalIgnoreCase)))
                    _pending.Add(new Failure(FailureKind.CalculationError, line.Trim(), DateTime.Now));
            }

            _resultsOffset = stream.Length;
        }
        catch (IOException)
        {
            // Prime95 holds the file briefly while writing; the next poll picks it up.
        }
    }

    public void Suspend()
    {
        if (!IsAlive) return;
        try { Native.NtSuspendProcess(_process.Handle); } catch { }
    }

    public void Resume()
    {
        try { Native.NtResumeProcess(_process.Handle); } catch { }
    }

    public void Pause(TimeSpan duration)
    {
        if (!IsAlive) return;

        try
        {
            Native.NtSuspendProcess(_process.Handle);
            Thread.Sleep(duration);
        }
        catch { /* process died mid-pause; the resume below is then a no-op */ }
        finally
        {
            try { Native.NtResumeProcess(_process.Handle); } catch { }
        }
    }

    public void Stop()
    {
        try { if (!_process.HasExited) _process.Kill(entireProcessTree: true); }
        catch { /* already gone */ }
    }

    public void Dispose()
    {
        Stop();
        _process.Dispose();
        _jail?.Dispose();
    }
}
