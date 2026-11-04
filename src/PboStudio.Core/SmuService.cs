using ZenStates.Core;

namespace PboStudio.Core;

/// <summary>How much of the Curve Optimizer state this CPU will actually reveal.</summary>
public enum CurveOptimizerSupport
{
    /// <summary>Not an AMD Ryzen, or the SMU is unreachable.</summary>
    None,
    /// <summary>The CPU reports its live margins, including what the BIOS set.</summary>
    Full,
}

public sealed record PboSnapshot(
    float Scalar,
    uint BoostMhz,
    float? PptLimitW = null, float? PptNowW = null,
    float? TdcLimitA = null, float? TdcNowA = null,
    float? EdcLimitA = null, float? EdcNowA = null,
    float? TempLimitC = null, float? TempNowC = null,
    uint PmTableVersion = 0,
    string? LayoutName = null,
    bool LayoutVerified = true)
{
    public bool HasLimits => PptLimitW is > 0;

    /// <summary>
    /// Whether this processor's power table is one the program knows how to read. False means
    /// the live readings are absent because they cannot be read, not because they are zero.
    /// </summary>
    public bool TableKnown => LayoutName is not null;
}

/// <summary>
/// Everything the app does to the processor goes through here.
/// Thread-safe wrapper around ZenStates.Core / PawnIO IOCTL calls.
/// </summary>
public sealed class SmuService : ISmuService
{
    private readonly Cpu? _cpu;
    private readonly object _smuLock = new();

    public string? InitError { get; }

    /// <summary>
    /// A <c>Cpu</c> object alone is not enough. Without the PawnIO driver ZenStates still
    /// constructs one, but it reports a topology of zero cores - which then divided by zero
    /// in <see cref="MaskFor"/> and took the whole app down at startup instead of degrading
    /// to the stress-test-only mode the UI already advertises.
    /// </summary>
    public bool IsAvailable => _cpu is not null && PhysicalCores > 0;

    public string CpuName { get; } = "unknown";
    public int PhysicalCores { get; }
    public CurveOptimizerSupport Support { get; } = CurveOptimizerSupport.None;

    /// <summary>Zero means ZenStates could not read the CCD fuse; that is not a count.</summary>
    public int? ReportedCcdCount { get; }

    /// <summary>Read once at startup from MSR 0xC00102B3; the values do not change at runtime.</summary>
    public IReadOnlyList<int>? CorePerformanceRanking { get; }

    public SmuService()
    {
        try
        {
            _cpu = new Cpu();
        }
        catch (Exception ex)
        {
            InitError = ex.InnerException?.Message ?? ex.Message;
            return;
        }

        CpuName = _cpu.info.cpuName?.Trim() ?? "unknown";
        PhysicalCores = (int)_cpu.info.topology.physicalCores;
        ReportedCcdCount = _cpu.info.topology.ccds > 0 ? (int)_cpu.info.topology.ccds : null;
        CorePerformanceRanking = _cpu.info.topology.performanceOfCore is { Length: > 0 } perf
            ? [.. perf.Select(v => (int)v)]
            : null;
        Support = CurveOptimizerSupport.Full;
    }

    /// <summary>
    /// True when every core reports 0. Either nothing is set, or the board never passed its
    /// Curve Optimizer settings to the SMU.
    /// </summary>
    public bool ReadsAllZero()
    {
        lock (_smuLock)
        {
            if (_cpu is null) return false;

            for (int i = 0; i < PhysicalCores; i++)
            {
                if (MaskFor(i) is not { } mask) return false;
                uint? raw = _cpu.GetPsmMarginSingleCore(mask);
                if (raw is not (null or 0)) return false;
            }

            return true;
        }
    }

    private uint? MaskFor(int core)
    {
        if (_cpu is null || PhysicalCores <= 0) return null;

        uint ccds = Math.Max(1u, _cpu.info.topology.ccds);
        uint perCcd = (uint)PhysicalCores / ccds;
        if (perCcd == 0) return null;

        return _cpu.MakeCoreMask((uint)core % perCcd, (uint)core / perCcd, 0);
    }

    /// <summary>
    /// The margin this CPU reports for a core, or null when it will not say.
    /// Thread-safe via _smuLock.
    /// </summary>
    public int? ReadCurveOptimizer(int core)
    {
        lock (_smuLock)
        {
            if (_cpu is null || MaskFor(core) is not { } mask) return null;

            uint? raw = _cpu.GetPsmMarginSingleCore(mask);
            return raw is null ? null : unchecked((int)raw.Value);
        }
    }

    public bool WriteCurveOptimizer(int core, int margin)
    {
        lock (_smuLock)
        {
            return _cpu is not null
                && MaskFor(core) is { } mask
                && _cpu.SetPsmMarginSingleCore(mask, margin);
        }
    }

    public bool WriteCurveOptimizerAllCores(int margin)
    {
        lock (_smuLock)
        {
            return _cpu is not null && _cpu.SetPsmMarginAllCores(margin);
        }
    }

    /// <summary>
    /// The live readings, read from wherever this processor generation keeps them.
    /// <para>
    /// The positions used to be a fixed set - the Zen 3 ones - applied to every processor. On a
    /// Zen 4 part they hold something else and came out as zero. A table version this program
    /// does not recognise now yields a snapshot with no readings and <see cref="PboSnapshot.TableKnown"/>
    /// false, which the window can say in words.
    /// </para>
    /// </summary>
    public PboSnapshot? ReadPbo()
    {
        lock (_smuLock)
        {
            if (_cpu is null) return null;

            float scalar = 1f;
            try { scalar = _cpu.GetPBOScalar(); } catch { }

            uint boost = 0;
            try { boost = _cpu.GetFMax(); } catch { }

            uint version = 0;
            try { version = _cpu.RyzenSmu?.PmTableVersion ?? 0; } catch { }

            var layout = PmTableLayout.For(version);
            if (layout is null)
                return new PboSnapshot(scalar, boost, PmTableVersion: version);

            float[]? t = null;
            try
            {
                _cpu.RefreshPowerTable();
                t = _cpu.powerTable?.Table;
            }
            catch { }

            var reading = t is null ? null : layout.Read(t, scalar, boost);

            return (reading ?? new PboSnapshot(scalar, boost)) with
            {
                PmTableVersion = version,
                LayoutName = layout.Name,
                LayoutVerified = layout.Verified,
            };
        }
    }

    /// <summary>
    /// The SMU's own report, including a dump of the power table. When a processor's layout is
    /// not known this is what makes it knowable: the contents can be compared against another
    /// monitoring tool to find which positions hold the temperature and power.
    /// </summary>
    public string? PmTableReport()
    {
        lock (_smuLock)
        {
            if (_cpu?.RyzenSmu is not { } smu) return null;

            try { return smu.GetReport(); }
            catch (Exception ex) { return $"Could not produce the report: {ex.Message}"; }
        }
    }

    public void Dispose()
    {
        lock (_smuLock)
        {
            _cpu?.Dispose();
        }
    }
}
