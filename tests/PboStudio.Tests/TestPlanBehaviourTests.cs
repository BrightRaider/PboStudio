using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

/// <summary>
/// The plan-level behaviours added alongside the CoreCycler parity pass: automatic runtime,
/// core-pair ordering, the SMT spread mask, and parking untested cores.
/// </summary>
public class TestPlanBehaviourTests
{
    // ── automatic runtime ────────────────────────────────────────

    [Fact]
    public void AutoRuntimeIsRecognisedFromTheSentinel()
    {
        Assert.True(new TestPlan { RuntimePerCore = TestPlan.AutoRuntime }.IsAutoRuntime);
        Assert.False(new TestPlan { RuntimePerCore = TimeSpan.FromMinutes(6) }.IsAutoRuntime);
    }

    [Fact]
    public void AutoRuntimeCarriesACapSoAMissingCycleReportCannotPinACoreForever()
    {
        var plan = new TestPlan { RuntimePerCore = TestPlan.AutoRuntime };
        Assert.True(plan.AutoRuntimeCap > TimeSpan.Zero);
    }

    // ── SMT mask ─────────────────────────────────────────────────

    private static PhysicalCore SmtCore() => new(0, 0b11, [0, 1]);
    private static PhysicalCore NoSmtCore() => new(0, 0b1, [0]);

    [Fact]
    public void OneThreadPinsToTheFirstSiblingByDefault()
    {
        Assert.Equal((nuint)0b01, SmtCore().MaskFor(threads: 1));
    }

    [Fact]
    public void TwoThreadsAlwaysCoverBothSiblings()
    {
        Assert.Equal((nuint)0b11, SmtCore().MaskFor(threads: 2));
        Assert.Equal((nuint)0b11, SmtCore().MaskFor(threads: 2, spreadAcrossSmt: true));
    }

    [Fact]
    public void SpreadingOneThreadWidensTheMaskToBothSiblings()
    {
        Assert.Equal((nuint)0b11, SmtCore().MaskFor(threads: 1, spreadAcrossSmt: true));
    }

    [Fact]
    public void SpreadingHasNoEffectWithoutSmt()
    {
        Assert.Equal((nuint)0b1, NoSmtCore().MaskFor(threads: 1, spreadAcrossSmt: true));
    }

    // ── core pairs ───────────────────────────────────────────────

    [Fact]
    public void CorePairsVisitsEveryOrderedHandover()
    {
        var order = OrderFor(CoreOrder.CorePairs, cores: 3);

        // 3 cores produce 6 ordered pairs, each contributing two slots.
        Assert.Equal(12, order.Count);

        var handovers = new HashSet<(int, int)>();
        for (int i = 0; i + 1 < order.Count; i += 2) handovers.Add((order[i], order[i + 1]));

        Assert.Equal(6, handovers.Count);
        Assert.DoesNotContain(handovers, p => p.Item1 == p.Item2);
    }

    // CorePairs is left out here on purpose: it is covered above, and its slot count grows
    // quadratically while the runner spends a second per slot.
    [Theory]
    [InlineData(CoreOrder.Sequential)]
    [InlineData(CoreOrder.Alternate)]
    [InlineData(CoreOrder.Random)]
    public void EveryOrderStillCoversEveryCore(CoreOrder mode)
    {
        Assert.Equal(4, OrderFor(mode, cores: 4).Distinct().Count());
    }

    /// <summary>
    /// Drives a runner with a zero-length slot so it walks the order without doing any work,
    /// and records which cores it visited.
    /// </summary>
    private static List<int> OrderFor(CoreOrder mode, int cores)
    {
        var cpu = Enumerable.Range(0, cores)
            .Select(i => new PhysicalCore(i, (nuint)1 << i, [i]))
            .ToList();

        var visited = new List<int>();
        var plan = new TestPlan
        {
            Order = mode,
            RuntimePerCore = TimeSpan.FromMilliseconds(1),
            MaxIterations = 1,
            DelayBetweenCores = TimeSpan.Zero,
            SuspendPeriodically = false,
        };

        var runner = new TestRunner(new RecordingEngine(visited), cpu, plan);
        runner.RunAsync(CancellationToken.None).GetAwaiter().GetResult();
        return visited;
    }

    private sealed class RecordingEngine(List<int> visited) : IStressEngine
    {
        public EngineKind Kind => EngineKind.Prime95;
        public string DisplayName => "recording";
        public bool IsAvailable => true;

        public IStressSession Start(PhysicalCore core, int threads)
        {
            visited.Add(core.Index);
            return new IdleSession();
        }
    }

    private sealed class IdleSession : IStressSession
    {
        public bool IsAlive => true;
        public TimeSpan CpuTime { get; private set; }
        public int WorkloadCyclesCompleted => 0;

        public IReadOnlyList<Failure> DrainFailures()
        {
            CpuTime += TimeSpan.FromSeconds(1);
            return [];
        }

        public void Pause(TimeSpan duration) { }
        public void Stop() { }
        public void Dispose() { }
    }
}
