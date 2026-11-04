using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

/// <summary>
/// Three things the runner got wrong, each of which loses something the reader cannot see:
///
///  - A stop in the middle of a run left every other core parked at 0, with the table claiming
///    otherwise: the undervolt silently gone.
///  - A processor that refuses a deeper value was "tested at whatever it holds" and the pass was
///    booked against the value that was asked for, so the search walked to the floor on passes at
///    a depth that never ran and locked a figure nobody tested.
///  - The end-of-run check compared a count of cores with the length of the order, which never came
///    true for pair orders and came true too early for a subset.
/// </summary>
public class RunnerSafetyTests
{
    // -- fakes -------------------------------------------------------------------------------

    private sealed class Session : IStressSession
    {
        public bool IsAlive => true;
        public TimeSpan CpuTime { get; private set; }
        public int WorkloadCyclesCompleted => 0;

        public IReadOnlyList<Failure> DrainFailures()
        {
            CpuTime += TimeSpan.FromSeconds(1);   // keeps the idle check quiet
            return [];
        }

        public void Pause(TimeSpan duration) { }
        public void Suspend() { }
        public void Resume() { }
        public void Stop() { }
        public void Dispose() { }
    }

    private sealed class Engine(Action<PhysicalCore>? onStart = null) : IStressEngine
    {
        public List<int> Started { get; } = [];
        public EngineKind Kind => EngineKind.Prime95;
        public string DisplayName => "fake";
        public bool IsAvailable => true;

        public IStressSession Start(PhysicalCore core, int threads)
        {
            Started.Add(core.Index);
            onStart?.Invoke(core);
            return new Session();
        }
    }

    private static IReadOnlyList<PhysicalCore> Cores(int n) =>
        [.. Enumerable.Range(0, n).Select(i => new PhysicalCore(i, (nuint)3 << (i * 2), [i * 2, i * 2 + 1]))];

    private static TestPlan Plan(
        Dictionary<int, int> margins,
        Func<int, int, bool>? apply = null,
        AutoTunerMode? tuner = null) => new()
    {
        RuntimePerCore = TimeSpan.FromMilliseconds(20),
        MaxIterations = 1,
        Order = CoreOrder.Sequential,
        DelayBetweenCores = TimeSpan.Zero,
        SuspendPeriodically = false,
        AutoTuner = tuner,
        MaxNegativeMargin = -50,
        Guardband = 0,
        InitialCoreMargins = margins,
        ApplyMargin = apply,
    };

    // -- R1: stopping must give the cores back -----------------------------------------------

    /// <summary>
    /// With "test only this core" the others are parked at 0 for each slot. Pressing stop in the
    /// middle skipped the restore, and the processor kept 0 on every other core while the table
    /// showed their values.
    /// </summary>
    [Fact]
    public async Task StoppingInTheMiddleOfAnIsolatedRunRestoresTheParkedCores()
    {
        var margins = new Dictionary<int, int> { [0] = -10, [1] = -20, [2] = -25 };
        var hardware = new Dictionary<int, int>(margins);

        // The stop lands while the runner is waiting out the slot, which is where it spends nearly
        // all of its time. That is the path that throws; cancelling before the wait begins just
        // ends the loop normally and never exercised the bug.
        using var cts = new CancellationTokenSource();
        cts.CancelAfter(TimeSpan.FromMilliseconds(300));

        var plan = Plan(margins, (core, value) => { hardware[core] = value; return true; }) with
        {
            IsolateTestedCore = true,
            RuntimePerCore = TimeSpan.FromSeconds(5),
        };

        await Assert.ThrowsAnyAsync<OperationCanceledException>(
            () => new TestRunner(new Engine(), Cores(3), plan).RunAsync(cts.Token));

        // Parked at 0 for the slot, then given back.
        Assert.Equal(-20, hardware[1]);
        Assert.Equal(-25, hardware[2]);
    }

    [Fact]
    public async Task AnEngineThatCannotStartStillGivesTheParkedCoresBack()
    {
        var margins = new Dictionary<int, int> { [0] = -10, [1] = -20 };
        var hardware = new Dictionary<int, int>(margins);

        var engine = new Engine(_ => throw new FileNotFoundException("prime95.exe missing"));
        var plan = Plan(margins, (core, value) => { hardware[core] = value; return true; }) with
        {
            IsolateTestedCore = true,
        };

        await Assert.ThrowsAsync<FileNotFoundException>(
            () => new TestRunner(engine, Cores(2), plan).RunAsync());

        Assert.Equal(-20, hardware[1]);
    }

    // -- R2: when is there nothing left ------------------------------------------------------

    /// <summary>
    /// A pair order visits every core many times, so "excluded cores >= entries in the order" never
    /// came true and a finished run spun through the rest of its passes doing nothing.
    /// </summary>
    [Fact]
    public async Task APairOrderEndsOnceEveryCoreIsLocked()
    {
        // At the limit every core locks on its first pass.
        var margins = new Dictionary<int, int> { [0] = -50, [1] = -50, [2] = -50 };
        int iterations = 0;

        var plan = Plan(margins, tuner: AutoTunerMode.Grob) with
        {
            Order = CoreOrder.CorePairs,
            MaxIterations = 40,
        };

        var runner = new TestRunner(new Engine(), Cores(3), plan);
        runner.Progress += e => { if (e is TestEvent.IterationStarted) iterations++; };

        await runner.RunAsync();

        Assert.Equal(1, iterations);
    }

    /// <summary>
    /// The opposite mistake: cores left out of a custom order were counted as excluded, so a run
    /// over a subset stopped after its first pass however many were asked for.
    /// </summary>
    [Fact]
    public async Task ACustomOrderOverASubsetRunsAllTheRequestedPasses()
    {
        var margins = new Dictionary<int, int> { [0] = -10, [1] = -10, [2] = -10, [3] = -10 };
        int iterations = 0;

        var plan = Plan(margins) with
        {
            Order = CoreOrder.Custom,
            CustomOrder = [0, 1],
            CoresToIgnore = new HashSet<int> { 2, 3 },
            MaxIterations = 3,
        };

        var runner = new TestRunner(new Engine(), Cores(4), plan);
        runner.Progress += e => { if (e is TestEvent.IterationStarted) iterations++; };

        await runner.RunAsync();

        Assert.Equal(3, iterations);
    }

    // -- R3: a processor that will not go that deep ------------------------------------------

    /// <summary>
    /// A board that takes -30 and nothing deeper, with the search allowed down to -50. The write of
    /// -33 is refused. The old behaviour tested whatever the CPU held and booked the pass at -33,
    /// then -36, and so on down to a locked figure that had never been applied.
    /// </summary>
    [Fact]
    public async Task ARefusedDeeperValueLocksTheCoreWhereItLastHeldUp()
    {
        var margins = new Dictionary<int, int> { [0] = -27 };
        var written = new List<int>();

        bool Apply(int core, int value)
        {
            written.Add(value);
            return value >= -30;   // the processor stops at -30
        }

        var tuned = new List<AutoTunerStepResult>();
        var engine = new Engine();

        var plan = Plan(margins, Apply, AutoTunerMode.Grob) with { MaxIterations = 6 };

        var runner = new TestRunner(engine, Cores(1), plan);
        runner.Progress += e => { if (e is TestEvent.CoreAutoTuned t) tuned.Add(t.Result); };

        await runner.RunAsync();

        // -27 held, -30 held; -33 was refused and never put under load.
        Assert.Equal(2, engine.Started.Count);

        var lastStep = tuned[^1];
        Assert.True(lastStep.CoreLocked);
        Assert.Equal(-30, lastStep.NextMargin);   // guardband is 0 in this plan
        Assert.Contains(-33, written);
    }

    [Fact]
    public async Task TheLockedValueKeepsItsHeadroomLikeAnyOther()
    {
        var margins = new Dictionary<int, int> { [0] = -30 };
        var tuned = new List<AutoTunerStepResult>();

        var plan = Plan(margins, (_, v) => v >= -30, AutoTunerMode.Grob) with
        {
            MaxIterations = 6,
            Guardband = 3,
        };

        var runner = new TestRunner(new Engine(), Cores(1), plan);
        runner.Progress += e => { if (e is TestEvent.CoreAutoTuned t) tuned.Add(t.Result); };

        await runner.RunAsync();

        Assert.Equal(-27, tuned[^1].NextMargin);   // -30 plus three points of headroom
    }

    /// <summary>
    /// A core with nothing proven to fall back on - its very first value is refused - is left out.
    /// Testing it at whatever it happens to hold and booking the result against the asked value is
    /// the exact mistake this guards against.
    /// </summary>
    [Fact]
    public async Task ACoreWhoseFirstDeepValueIsRefusedIsSkippedNotTested()
    {
        var margins = new Dictionary<int, int> { [0] = -40 };
        var engine = new Engine();
        var infos = new List<string>();
        var passed = new List<int>();

        var plan = Plan(margins, (_, v) => v >= -30, AutoTunerMode.Grob);

        var runner = new TestRunner(engine, Cores(1), plan);
        runner.Progress += e =>
        {
            if (e is TestEvent.Info i) infos.Add(i.Message);
            if (e is TestEvent.CorePassed p) passed.Add(p.Core);
        };

        await runner.RunAsync();

        Assert.Empty(engine.Started);
        Assert.Empty(passed);
        Assert.Contains(infos, m => m.Contains("skipped", StringComparison.OrdinalIgnoreCase));
    }

    /// <summary>
    /// Inside the standard range a refused write keeps its old meaning - tested at what the CPU
    /// holds, the run carries on. That path is the verified one and is not changed.
    /// </summary>
    [Fact]
    public async Task ARefusalInsideTheStandardRangeIsStillOnlyReported()
    {
        var margins = new Dictionary<int, int> { [0] = -25 };
        var engine = new Engine();

        var plan = Plan(margins, (_, _) => false, AutoTunerMode.Grob);

        await new TestRunner(engine, Cores(1), plan).RunAsync();

        Assert.Single(engine.Started);
    }

    [Fact]
    public async Task WithoutAnAutoTunerARefusedDeepValueDoesNotSkipTheCore()
    {
        var margins = new Dictionary<int, int> { [0] = -40 };
        var engine = new Engine();

        var plan = Plan(margins, (_, _) => false, tuner: null);

        await new TestRunner(engine, Cores(1), plan).RunAsync();

        Assert.Single(engine.Started);
    }
}
