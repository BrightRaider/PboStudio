using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

/// <summary>
/// The auto-tuner used to reason about a margin the processor was never actually set to: the
/// table said -30, the CPU still held the BIOS value, the core passed, and -30 got locked in as
/// validated without ever having been under load. These tests pin the ordering that fixes it.
/// </summary>
public class AutoTunerApplyMarginTests
{
    private sealed class NoOpSession : IStressSession
    {
        public bool IsAlive => true;
        public TimeSpan CpuTime { get; private set; }
        private readonly IReadOnlyList<Failure> _failures;

        public NoOpSession(IReadOnlyList<Failure> failures) => _failures = failures;

        /// <summary>These fakes never report a completed sweep, so a plan using the
        /// automatic runtime falls back to its cap - which is what the tests want.</summary>
        public int WorkloadCyclesCompleted => 0;

        public IReadOnlyList<Failure> DrainFailures()
        {
            // Keep CpuTime climbing so the runner does not judge the core to have gone idle.
            CpuTime += TimeSpan.FromSeconds(1);
            return _failures;
        }

        public void Pause(TimeSpan duration) { }
        public void Stop() { }
        public void Dispose() { }
    }

    private sealed class FakeEngine : IStressEngine
    {
        private readonly IReadOnlyList<Failure> _failures;
        public FakeEngine(IReadOnlyList<Failure>? failures = null) => _failures = failures ?? [];

        public EngineKind Kind => EngineKind.Prime95;
        public string DisplayName => "fake";
        public bool IsAvailable => true;

        public IStressSession Start(PhysicalCore core, int threads) => new NoOpSession(_failures);
    }

    private static IReadOnlyList<PhysicalCore> Cores(int count)
    {
        var list = new List<PhysicalCore>();
        for (int i = 0; i < count; i++)
            list.Add(new PhysicalCore(i, (nuint)3 << (i * 2), [i * 2, i * 2 + 1]));
        return list;
    }

    private static TestPlan BasePlan(Func<int, int, bool>? apply, IReadOnlyDictionary<int, int> initial) => new()
    {
        RuntimePerCore = TimeSpan.FromMilliseconds(20),
        MaxIterations = 1,
        Order = CoreOrder.Sequential,
        DelayBetweenCores = TimeSpan.Zero,
        SuspendPeriodically = false,
        AutoTuner = AutoTunerMode.Grob,
        MaxNegativeMargin = -30,
        InitialCoreMargins = initial,
        ApplyMargin = apply,
    };

    [Fact]
    public async Task PlannedMarginIsWrittenToHardwareBeforeTheCoreIsTested()
    {
        var written = new List<(int Core, int Margin)>();
        var initial = new Dictionary<int, int> { [0] = -30, [1] = -30 };

        var runner = new TestRunner(new FakeEngine(), Cores(2), BasePlan((c, m) =>
        {
            written.Add((c, m));
            return true;
        }, initial));

        await runner.RunAsync();

        // Both cores must have been set to the value the table showed, not left as they were.
        Assert.Contains((0, -30), written);
        Assert.Contains((1, -30), written);
    }

    [Fact]
    public async Task TheWriteHappensBeforeTheEngineStarts()
    {
        var sequence = new List<string>();
        var initial = new Dictionary<int, int> { [0] = -20 };

        var engine = new OrderRecordingEngine(sequence);
        var plan = BasePlan((c, m) => { sequence.Add($"apply:{c}={m}"); return true; }, initial);

        await new TestRunner(engine, Cores(1), plan).RunAsync();

        Assert.Equal("apply:0=-20", sequence[0]);
        Assert.Equal("start:0", sequence[1]);
    }

    private sealed class OrderRecordingEngine : IStressEngine
    {
        private readonly List<string> _sequence;
        public OrderRecordingEngine(List<string> sequence) => _sequence = sequence;

        public EngineKind Kind => EngineKind.Prime95;
        public string DisplayName => "fake";
        public bool IsAvailable => true;

        public IStressSession Start(PhysicalCore core, int threads)
        {
            _sequence.Add($"start:{core.Index}");
            return new NoOpSession([]);
        }
    }

    [Fact]
    public async Task AFailedWriteDoesNotAbortTheRun()
    {
        // Refusing to write is worth reporting, but the run still has to finish rather than
        // leave the user with a half-tested processor.
        var initial = new Dictionary<int, int> { [0] = -25 };
        var infos = new List<string>();

        var runner = new TestRunner(new FakeEngine(), Cores(1), BasePlan((_, _) => false, initial));
        runner.Progress += ev => { if (ev is TestEvent.Info i) infos.Add(i.Message); };

        var failures = await runner.RunAsync();

        Assert.Empty(failures);
        Assert.Contains(infos, m => m.Contains("could not set", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public async Task WithoutTheHookTheHardwareIsLeftAlone()
    {
        // Null hook is what the rest of the test suite relies on, and what a run without the
        // driver has to fall back to.
        var initial = new Dictionary<int, int> { [0] = -30 };
        var runner = new TestRunner(new FakeEngine(), Cores(1), BasePlan(null, initial));

        var failures = await runner.RunAsync();

        Assert.Empty(failures);
    }

    [Fact]
    public async Task EachStepIsAppliedBeforeItsOwnPass()
    {
        // Two passes: the second must be applied at the margin the first pass produced.
        var written = new List<int>();
        var initial = new Dictionary<int, int> { [0] = -10 };

        var plan = BasePlan((_, m) => { written.Add(m); return true; }, initial) with
        {
            MaxIterations = 2,
        };

        await new TestRunner(new FakeEngine(), Cores(1), plan).RunAsync();

        Assert.Equal(2, written.Count);
        Assert.Equal(-10, written[0]);
        // Passing at -10 in coarse mode steps down by 3 before the next pass.
        Assert.Equal(-13, written[1]);
    }
}
