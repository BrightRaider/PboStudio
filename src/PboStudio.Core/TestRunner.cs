using System.Diagnostics;

namespace PboStudio.Core;

public enum CoreOrder { Sequential, Alternate, Random, Custom }

public sealed record TestPlan
{
    public TimeSpan RuntimePerCore { get; init; } = TimeSpan.FromMinutes(6);
    public int Threads { get; init; } = 1;
    public int MaxIterations { get; init; } = int.MaxValue;
    public CoreOrder Order { get; init; } = CoreOrder.Sequential;
    public IReadOnlyList<int> CustomOrder { get; init; } = [];
    public IReadOnlySet<int> CoresToIgnore { get; init; } = new HashSet<int>();
    public bool StopOnError { get; init; }
    public bool SkipCoreOnError { get; init; } = true;
    public bool TreatWheaWarningAsError { get; init; } = true;
    public TimeSpan DelayBetweenCores { get; init; } = TimeSpan.FromSeconds(2);
    public AutoTunerMode? AutoTuner { get; init; }
    public int MaxNegativeMargin { get; init; } = AutoTunerService.DefaultBiosLimit;
    public IReadOnlyDictionary<int, int>? InitialCoreMargins { get; init; }

    /// <summary>
    /// Writes a core's Curve Optimizer margin to the hardware, returning whether it took.
    /// Called immediately before that core is measured.
    /// <para>
    /// Without this the auto-tuner reasoned about one value while the processor ran another:
    /// the table said -30, the CPU still held whatever the BIOS set, the core passed, and the
    /// tuner locked -30 in as "validated" although it had never been under load.
    /// </para>
    /// Null leaves the hardware untouched, which is what the tests want.
    /// </summary>
    public Func<int, int, bool>? ApplyMargin { get; init; }
    public bool IsGerman { get; init; }

    /// <summary>Interrupt the load now and then so the core has to boost back up.</summary>
    public bool SuspendPeriodically { get; init; } = true;
    public TimeSpan SuspendEvery { get; init; } = TimeSpan.FromSeconds(30);
    public TimeSpan SuspendFor { get; init; } = TimeSpan.FromSeconds(1);
}

public abstract record TestEvent
{
    public sealed record RunStarted(int CoreCount, TimeSpan PerCore) : TestEvent;
    public sealed record IterationStarted(int Iteration) : TestEvent;
    public sealed record CoreStarted(int Core, int Iteration) : TestEvent;
    public sealed record CorePassed(int Core, TimeSpan Duration) : TestEvent;
    public sealed record CoreFailed(int Core, Failure Failure) : TestEvent;
    public sealed record CoreAutoTuned(int Core, AutoTunerStepResult Result) : TestEvent;
    public sealed record Info(string Message) : TestEvent;
    public sealed record RunFinished(IReadOnlyDictionary<int, List<Failure>> Failures) : TestEvent;
}

/// <summary>
/// Walks the cores, loads each one in turn, and decides whether it held up.
///
/// A core is judged by four independent signals, because no single one catches everything:
/// the engine's own error reporting, the worker process disappearing, machine check events,
/// and the core quietly going idle while it was supposed to be under load.
/// </summary>
public sealed class TestRunner
{
    private readonly IStressEngine _engine;
    private readonly IReadOnlyList<PhysicalCore> _cores;
    private readonly TestPlan _plan;

    public event Action<TestEvent>? Progress;

    public TestRunner(IStressEngine engine, IReadOnlyList<PhysicalCore> cores, TestPlan plan)
    {
        _engine = engine;
        _cores = cores;
        _plan = plan;
    }

    public async Task<IReadOnlyDictionary<int, List<Failure>>> RunAsync(CancellationToken ct = default)
    {
        var failures = new Dictionary<int, List<Failure>>();
        var excluded = new HashSet<int>(_plan.CoresToIgnore);
        var currentMargins = _plan.InitialCoreMargins != null
            ? new Dictionary<int, int>(_plan.InitialCoreMargins)
            : new Dictionary<int, int>();

        // What the tuner has learned per core. Without this the search has no memory and
        // cannot tell "never passed" from "passed lower down and then failed".
        var tunerState = new Dictionary<int, AutoTunerCoreState>();
        foreach (var c in _cores)
            if (!currentMargins.ContainsKey(c.Index)) currentMargins[c.Index] = 0;

        var order = BuildOrder();
        Emit(new TestEvent.RunStarted(order.Count(c => !excluded.Contains(c)), _plan.RuntimePerCore));

        using var whea = new WheaWatcher();
        var wheaHits = new List<WheaEvent>();
        whea.Raised += e => { lock (wheaHits) wheaHits.Add(e); };
        whea.Start();

        for (int iteration = 1; iteration <= _plan.MaxIterations && !ct.IsCancellationRequested; iteration++)
        {
            Emit(new TestEvent.IterationStarted(iteration));

            foreach (int coreIndex in order)
            {
                if (ct.IsCancellationRequested) break;
                if (excluded.Contains(coreIndex)) continue;

                lock (wheaHits) wheaHits.Clear();

                ApplyPlannedMargin(coreIndex, currentMargins);

                var found = await TestCoreAsync(_cores[coreIndex], iteration, wheaHits, ct);

                if (found.Count > 0)
                {
                    if (!failures.TryGetValue(coreIndex, out var list))
                        failures[coreIndex] = list = [];
                    list.AddRange(found);

                    foreach (var f in found)
                        Emit(new TestEvent.CoreFailed(coreIndex, f));

                    if (_plan.AutoTuner.HasValue)
                    {
                        int cur = currentMargins[coreIndex];
                        var res = AutoTunerService.CalculateNextStep(
                            coreIndex, cur, passed: false, _plan.AutoTuner.Value,
                            _plan.MaxNegativeMargin, _plan.IsGerman,
                            tunerState.GetValueOrDefault(coreIndex));
                        currentMargins[coreIndex] = res.NextMargin;
                        tunerState[coreIndex] = res.State;
                        Emit(new TestEvent.CoreAutoTuned(coreIndex, res));
                        if (res.CoreLocked) excluded.Add(coreIndex);
                    }
                    else
                    {
                        if (_plan.StopOnError)
                        {
                            Emit(new TestEvent.Info("Stopping: configured to halt on the first error."));
                            goto done;
                        }
                        if (_plan.SkipCoreOnError) excluded.Add(coreIndex);
                    }
                }
                else
                {
                    Emit(new TestEvent.CorePassed(coreIndex, _plan.RuntimePerCore));

                    if (_plan.AutoTuner.HasValue)
                    {
                        int cur = currentMargins[coreIndex];
                        var res = AutoTunerService.CalculateNextStep(
                            coreIndex, cur, passed: true, _plan.AutoTuner.Value,
                            _plan.MaxNegativeMargin, _plan.IsGerman,
                            tunerState.GetValueOrDefault(coreIndex));
                        currentMargins[coreIndex] = res.NextMargin;
                        tunerState[coreIndex] = res.State;
                        Emit(new TestEvent.CoreAutoTuned(coreIndex, res));
                        if (res.CoreLocked) excluded.Add(coreIndex);
                    }
                }

                if (_plan.DelayBetweenCores > TimeSpan.Zero)
                    await Task.Delay(_plan.DelayBetweenCores, ct).ConfigureAwait(false);
            }

            if (excluded.Count >= order.Count)
            {
                Emit(new TestEvent.Info("Every core has been excluded or locked; nothing left to test."));
                break;
            }
        }

    done:
        Emit(new TestEvent.RunFinished(failures));
        return failures;
    }

    private async Task<List<Failure>> TestCoreAsync(
        PhysicalCore core, int iteration, List<WheaEvent> wheaHits, CancellationToken ct)
    {
        Emit(new TestEvent.CoreStarted(core.Index, iteration));

        var found = new List<Failure>();
        using var session = _engine.Start(core, _plan.Threads);

        var clock = Stopwatch.StartNew();
        var lastCpu = session.CpuTime;
        var lastCheck = clock.Elapsed;
        var nextSuspend = _plan.SuspendEvery;

        // Give the engine a moment to spin up before judging how busy the core is.
        var graceUntil = TimeSpan.FromSeconds(10);

        while (clock.Elapsed < _plan.RuntimePerCore && !ct.IsCancellationRequested)
        {
            await Task.Delay(1000, ct).ConfigureAwait(false);

            found.AddRange(session.DrainFailures());

            lock (wheaHits)
            {
                foreach (var hit in wheaHits)
                {
                    if (hit.IsWarning && !_plan.TreatWheaWarningAsError) continue;
                    found.Add(new Failure(FailureKind.MachineCheck, hit.Description, hit.Time));
                }
                wheaHits.Clear();
            }

            var now = clock.Elapsed;

            if (_plan.SuspendPeriodically && now >= nextSuspend)
            {
                session.Pause(_plan.SuspendFor);
                nextSuspend = clock.Elapsed + _plan.SuspendEvery;

                // The pause itself looks exactly like a core going idle, so restart the
                // occupancy baseline instead of reporting the interruption as a failure.
                lastCpu = session.CpuTime;
                lastCheck = clock.Elapsed;
                continue;
            }

            var cpu = session.CpuTime;
            if (now > graceUntil)
            {
                double wall = (now - lastCheck).TotalSeconds;
                double busy = (cpu - lastCpu).TotalSeconds;

                // Half the expected occupancy means the load is not where it should be.
                if (wall > 0 && busy / wall < _plan.Threads * 0.5)
                    found.Add(new Failure(FailureKind.WentIdle,
                        $"Core {core.Index} ran at {busy / wall:F2} of {_plan.Threads} expected thread(s).",
                        DateTime.Now));
            }
            lastCpu = cpu;
            lastCheck = now;

            if (found.Count > 0) break;
        }

        session.Stop();
        return found;
    }

    /// <summary>Puts the processor at the value about to be measured, and says so out loud.</summary>
    private void ApplyPlannedMargin(int coreIndex, IReadOnlyDictionary<int, int> currentMargins)
    {
        if (_plan.ApplyMargin is not { } apply) return;
        if (!currentMargins.TryGetValue(coreIndex, out int planned)) return;

        bool ok = apply(coreIndex, planned);
        Emit(new TestEvent.Info(ok
            ? (_plan.IsGerman
                ? $"Kern {coreIndex}: Curve Optimizer auf {planned} gesetzt, Test startet."
                : $"Core {coreIndex}: Curve Optimizer set to {planned}, starting test.")
            : (_plan.IsGerman
                ? $"Kern {coreIndex}: Curve Optimizer konnte nicht auf {planned} gesetzt werden — getestet wird der aktuell anliegende Wert."
                : $"Core {coreIndex}: could not set Curve Optimizer to {planned} — testing whatever the CPU currently holds.")));
    }

    private List<int> BuildOrder()
    {
        var all = Enumerable.Range(0, _cores.Count).ToList();

        return _plan.Order switch
        {
            CoreOrder.Sequential => all,
            CoreOrder.Random => [.. all.OrderBy(_ => Random.Shared.Next())],
            // Jump between halves so neighbouring cores never run back to back, which keeps
            // one hot spot from influencing the next core's result.
            CoreOrder.Alternate => [.. Interleave(all)],
            CoreOrder.Custom => [.. _plan.CustomOrder.Where(i => i >= 0 && i < _cores.Count)],
            _ => all,
        };
    }

    private static IEnumerable<int> Interleave(List<int> all)
    {
        int half = (all.Count + 1) / 2;
        for (int i = 0; i < half; i++)
        {
            yield return all[i];
            if (i + half < all.Count) yield return all[i + half];
        }
    }

    private void Emit(TestEvent e) => Progress?.Invoke(e);
}
