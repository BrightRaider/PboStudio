namespace PboStudio.Core;

/// <summary>
/// Abstraction for communicating with the AMD Ryzen System Management Unit (SMU).
/// Enables thread-safe hardware access and full mockability in unit tests.
/// </summary>
public interface ISmuService : IDisposable
{
    string? InitError { get; }
    bool IsAvailable { get; }
    string CpuName { get; }
    int PhysicalCores { get; }
    CurveOptimizerSupport Support { get; }

    /// <summary>
    /// CCD count as read from the processor's own fuse, or null when the driver could not
    /// read it. Authoritative when present - it is the only source that distinguishes a CCX
    /// from a CCD on Zen and Zen 2.
    /// </summary>
    int? ReportedCcdCount { get; }

    bool ReadsAllZero();
    int? ReadCurveOptimizer(int core);
    bool WriteCurveOptimizer(int core, int margin);
    bool WriteCurveOptimizerAllCores(int margin);
    PboSnapshot? ReadPbo();
}
