using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

/// <summary>
/// A multi-phase profile runs one <see cref="TestRunner"/> per phase. The auto-tuner's memory
/// used to live inside the runner, so it started from nothing at every phase boundary: a core
/// locked at -25 in phase 1 would pass at -25 in phase 2, look like a fresh descent, and be
/// stepped straight back down to the value phase 1 had just proved unstable.
/// </summary>
public class AutoTunerPhaseStateTests
{
    private sealed class Session(IReadOnlyList<Failure> failures) : IStressSession
    {
        public bool IsAlive => true;
        public TimeSpan CpuTime { get; private set; }

        /// <summary>These fakes never report a completed sweep, so a plan using the
        /// automatic runtime falls back to its cap - which is what the tests want.</summary>
        public int WorkloadCyclesCompleted => 0;

        public IReadOnlyList<Failure> DrainFailures()
        {
            CpuTime += TimeSpan.FromSeconds(1);
            return failures;
        }

        public void Pause(TimeSpan duration) { }
        public void Suspend() { }
        public void Resume() { }

        public void Stop() { }
        public void Dispose() { }
    }

    /// <summary>Fails while the margin is at or below <paramref name="failAtOrBelow"/>.</summary>
    private sealed class ThresholdEngine(Func<int> currentMargin, int failAtOrBelow) : IStressEngine
    {
        public EngineKind Kind => EngineKind.Prime95;
        public string DisplayName => "threshold";
        public bool IsAvailable => true;

        public IStressSession Start(PhysicalCore core, int threads) =>
            new Session(currentMargin() <= failAtOrBelow
                ? [new Failure(FailureKind.CalculationError, "unstable", DateTime.Now)]
                : []);
    }

    private static IReadOnlyList<PhysicalCore> OneCore() =>
        [new PhysicalCore(0, 3, [0, 1])];

    [Fact]
    public async Task ACoreLockedInPhaseOneIsNotReopenedInPhaseTwo()
    {
        // -28 fails, -25 holds. Phase 1 should therefore settle on -25 and stay there.
        var margins = new Dictionary<int, int> { [0] = -28 };
        var tunerState = new Dictionary<int, AutoTunerCoreState>();
        var locked = new HashSet<int>();
        int applied = -28;

        TestPlan PlanFor(IDictionary<int, int> seed) => new()
        {
            // No guardband here: this test is about a locked core staying out of later phases,
            // and the safety headroom would shift every expected value by 3.
            Guardband = 0,
            RuntimePerCore = TimeSpan.FromMilliseconds(20),
            MaxIterations = 3,
            Order = CoreOrder.Sequential,
            DelayBetweenCores = TimeSpan.Zero,
            SuspendPeriodically = false,
            AutoTuner = AutoTunerMode.Grob,
            MaxNegativeMargin = -30,
            InitialCoreMargins = new Dictionary<int, int>(seed),
            ApplyMargin = (_, m) => { applied = m; return true; },
            TunerState = tunerState,
            LockedCores = locked,
        };

        var engine = new ThresholdEngine(() => applied, failAtOrBelow: -28);

        // ── phase 1 ──
        var runner1 = new TestRunner(engine, OneCore(), PlanFor(margins));
        runner1.Progress += ev =>
        {
            if (ev is TestEvent.CoreAutoTuned t) margins[t.Core] = t.Result.NextMargin;
        };
        await runner1.RunAsync();

        Assert.Contains(0, locked);
        Assert.Equal(-25, margins[0]);

        // ── phase 2, fresh runner, same shared state ──
        int appliedInPhaseTwo = int.MinValue;
        var phase2 = PlanFor(margins) with
        {
            ApplyMargin = (_, m) => { applied = m; appliedInPhaseTwo = m; return true; },
        };
        var runner2 = new TestRunner(engine, OneCore(), phase2);
        runner2.Progress += ev =>
        {
            if (ev is TestEvent.CoreAutoTuned t) margins[t.Core] = t.Result.NextMargin;
        };
        await runner2.RunAsync();

        // The core was settled, so phase 2 must leave it alone entirely.
        Assert.Equal(int.MinValue, appliedInPhaseTwo);
        Assert.Equal(-25, margins[0]);
    }

    [Fact]
    public async Task WithoutSharedStateTheSecondPhaseWouldUndoTheFirst()
    {
        // Documents the old behaviour: no shared state, and the core is stepped back down to
        // the value that had already failed.
        var margins = new Dictionary<int, int> { [0] = -25 };
        int applied = -25;

        var plan = new TestPlan
        {
            // No guardband here: these tests are about the search state surviving a phase
            // boundary, and the safety headroom would shift every expected value by 3.
            Guardband = 0,
            RuntimePerCore = TimeSpan.FromMilliseconds(20),
            MaxIterations = 1,
            Order = CoreOrder.Sequential,
            DelayBetweenCores = TimeSpan.Zero,
            SuspendPeriodically = false,
            AutoTuner = AutoTunerMode.Grob,
            MaxNegativeMargin = -30,
            InitialCoreMargins = margins,
            ApplyMargin = (_, m) => { applied = m; return true; },
            // TunerState and LockedCores deliberately omitted.
        };

        var runner = new TestRunner(new ThresholdEngine(() => applied, failAtOrBelow: -28), OneCore(), plan);
        int next = -25;
        runner.Progress += ev => { if (ev is TestEvent.CoreAutoTuned t) next = t.Result.NextMargin; };

        await runner.RunAsync();

        Assert.Equal(-28, next);
    }

    [Fact]
    public async Task SharedStateCarriesTheKnownGoodValueAcrossPhases()
    {
        var tunerState = new Dictionary<int, AutoTunerCoreState>
        {
            [0] = new AutoTunerCoreState(LastKnownGood: -20),
        };
        var locked = new HashSet<int>();
        int applied = -23;

        var plan = new TestPlan
        {
            // No guardband here: these tests are about the search state surviving a phase
            // boundary, and the safety headroom would shift every expected value by 3.
            Guardband = 0,
            RuntimePerCore = TimeSpan.FromMilliseconds(20),
            MaxIterations = 1,
            Order = CoreOrder.Sequential,
            DelayBetweenCores = TimeSpan.Zero,
            SuspendPeriodically = false,
            AutoTuner = AutoTunerMode.Grob,
            MaxNegativeMargin = -30,
            InitialCoreMargins = new Dictionary<int, int> { [0] = -23 },
            ApplyMargin = (_, m) => { applied = m; return true; },
            TunerState = tunerState,
            LockedCores = locked,
        };

        var runner = new TestRunner(new ThresholdEngine(() => applied, failAtOrBelow: -23), OneCore(), plan);
        int next = 0;
        runner.Progress += ev => { if (ev is TestEvent.CoreAutoTuned t) next = t.Result.NextMargin; };

        await runner.RunAsync();

        // -23 fails; the value carried in from the earlier phase is known good, so it settles
        // there rather than climbing blindly.
        Assert.Equal(-20, next);
        Assert.Contains(0, locked);
    }
}
