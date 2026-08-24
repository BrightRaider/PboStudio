namespace PboStudio.Core;

public enum EngineKind { Prime95, YCruncher }

/// <summary>Why a core was marked as failed.</summary>
public enum FailureKind
{
    /// <summary>The engine itself reported a wrong calculation.</summary>
    CalculationError,
    /// <summary>The worker process died or vanished.</summary>
    ProcessGone,
    /// <summary>A machine check exception was logged.</summary>
    MachineCheck,
    /// <summary>The core stopped drawing load while it should have been busy.</summary>
    WentIdle,
}

public sealed record Failure(FailureKind Kind, string Detail, DateTime Time)
{
    public override string ToString() => $"{Kind}: {Detail}";
}

/// <summary>A stress engine running against one physical core.</summary>
public interface IStressSession : IDisposable
{
    bool IsAlive { get; }

    /// <summary>Total processor time consumed, used to notice a core that quietly went idle.</summary>
    TimeSpan CpuTime { get; }

    /// <summary>Failures the engine has reported since the last call.</summary>
    IReadOnlyList<Failure> DrainFailures();

    /// <summary>
    /// Freezes the load briefly so the core drops to idle and has to boost back up. Many Curve
    /// Optimizer values survive hours of uninterrupted load and fail on exactly this transition,
    /// where voltage and frequency move together.
    /// </summary>
    void Pause(TimeSpan duration);

    void Stop();
}

public interface IStressEngine
{
    EngineKind Kind { get; }
    string DisplayName { get; }

    /// <summary>True when the engine's files are present and it can actually run.</summary>
    bool IsAvailable { get; }

    /// <summary>
    /// Starts the load on one physical core. <paramref name="threads"/> of 1 uses a single
    /// logical processor, 2 uses both siblings of an SMT core.
    /// </summary>
    IStressSession Start(PhysicalCore core, int threads);
}
