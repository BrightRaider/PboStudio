using System.Diagnostics;

namespace PboStudio.Core;

public enum CoreOrder { Sequential, Alternate, Random, Custom, CorePairs }

public sealed record TestPlan
{
    /// <summary>
    /// Sentinel for <see cref="RuntimePerCore"/> meaning "hold this core until the engine has
    /// worked through its whole list once", rather than for a fixed number of minutes. A six
    /// minute slot only gets through a fraction of the FFT sizes in a preset like Huge, so a
    /// core can pass without ever having run most of the test it was nominally given.
    /// </summary>
    public static readonly TimeSpan AutoRuntime = TimeSpan.Zero;

    /// <summary>
    /// Hard ceiling for <see cref="AutoRuntime"/>. Cycle detection reads the engine's own
    /// output and an engine that never reports one would otherwise pin a core forever, so the
    /// automatic runtime is always bounded by a wall clock as well.
    /// </summary>
    public TimeSpan AutoRuntimeCap { get; init; } = TimeSpan.FromMinutes(60);

    public TimeSpan RuntimePerCore { get; init; } = TimeSpan.FromMinutes(6);
    public bool IsAutoRuntime => RuntimePerCore <= TimeSpan.Zero;
    public int Threads { get; init; } = 1;

    /// <summary>
    /// Run one worker, but let it roam across both SMT siblings instead of pinning it to the
    /// first. The scheduler then moves the load between the two logical processors, which
    /// produces transitions a fixed pin never creates. Ignored without SMT or at two threads.
    /// </summary>
    public bool SpreadSingleThreadAcrossSmt { get; init; }

    /// <summary>
    /// Park every core that is not under test at a margin of 0 for the duration of its slot.
    /// <para>
    /// Without this, a machine check raised by some other core's aggressive offset lands in the
    /// tested core's result and gets it backed off for a fault it never had. Costs one SMU
    /// write per core per slot and makes the attribution honest.
    /// </para>
    /// </summary>
    public bool IsolateTestedCore { get; init; }
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

    /// <summary>
    /// Per-core floor, from what previous runs observed. A core that once took the machine down
    /// at -30 must not be walked back to -30 to rediscover that, so its floor is one point
    /// safer. Missing entries fall back to <see cref="MaxNegativeMargin"/>.
    /// </summary>
    public IReadOnlyDictionary<int, int>? PerCoreFloor { get; init; }

    /// <summary>
    /// Headroom left between the value a core is measured to survive and the value it is locked
    /// at. A stress test finds the boundary; everyday use is not the stress test.
    /// </summary>
    public int Guardband { get; init; } = AutoTunerService.DefaultGuardband;

    /// <summary>
    /// Cores CPPC ranks highest. They boost furthest and carry the background work, so they get
    /// the extra headroom on top of <see cref="Guardband"/>.
    /// </summary>
    public IReadOnlySet<int> PreferredCores { get; init; } = new HashSet<int>();

    public int GuardbandFor(int core) => PreferredCores.Contains(core)
        ? Guardband + AutoTunerService.PreferredCoreExtraGuardband
        : Guardband;

    /// <summary>Floor actually used for one core.</summary>
    public int FloorFor(int core) =>
        PerCoreFloor is { } f && f.TryGetValue(core, out int limit)
            ? Math.Max(MaxNegativeMargin, limit)
            : MaxNegativeMargin;
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

    /// <summary>
    /// What the auto-tuner has learned, shared across phases. A profile like "Hybrid Ultimate"
    /// runs several phases, each with its own runner; keeping this inside the runner meant the
    /// search restarted from nothing at every phase boundary. A core locked at -25 in phase 1
    /// would then pass at -25 in phase 2, look like a fresh descent, and be stepped back down
    /// to the value phase 1 had just proved unstable.
    /// </summary>
    public IDictionary<int, AutoTunerCoreState>? TunerState { get; init; }

    /// <summary>Cores the auto-tuner has settled, likewise shared across phases.</summary>
    public ISet<int>? LockedCores { get; init; }
    public bool IsGerman { get; init; }

    /// <summary>Interrupt the load now and then so the core has to boost back up.</summary>
    public bool SuspendPeriodically { get; init; } = true;
    public TimeSpan SuspendEvery { get; init; } = TimeSpan.FromSeconds(30);
    public TimeSpan SuspendFor { get; init; } = TimeSpan.FromSeconds(1);

    /// <summary>How the load is interrupted.</summary>
    public TransientMode Transient { get; init; } = TransientMode.Periodic;

    /// <summary>Micro-burst: how long the load runs before each interruption.</summary>
    public TimeSpan BurstLoad { get; init; } = TimeSpan.FromMilliseconds(300);

    /// <summary>Micro-burst: how long the core is left idle between bursts.</summary>
    public TimeSpan BurstIdle { get; init; } = TimeSpan.FromMilliseconds(100);

    /// <summary>
    /// Share of wall time the load is actually running under the current transient settings.
    /// The idle check compares measured occupancy against this rather than against a solid
    /// 100 %, or micro-bursting would report every core as having gone idle.
    /// </summary>
    public double ExpectedDutyCycle
    {
        get
        {
            if (Transient != TransientMode.MicroBurst) return 1.0;

            double cycle = (BurstLoad + BurstIdle).TotalMilliseconds;
            return cycle <= 0 ? 1.0 : BurstLoad.TotalMilliseconds / cycle;
        }
    }
}

/// <summary>How the load is interrupted so the core has to boost back up.</summary>
public enum TransientMode
{
    /// <summary>A pause every N seconds. Roughly a dozen transitions in a six-minute slot.</summary>
    Periodic,

    /// <summary>
    /// Sub-second pulsing, hundreds of transitions per minute.
    /// <para>
    /// This exists because of a real failure: cores validated as stable under continuous load
    /// threw WHEA 18 (cache hierarchy error) during ordinary gaming. A game engine swings
    /// between idle and peak boost far faster than a periodic pause does, and it is the rate of
    /// voltage change on the way back up that a too-aggressive curve cannot follow. Continuous
    /// full load never produces that edge, so a value can pass for hours and still fail in use.
    /// </para>
    /// </summary>
    MicroBurst,
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
        if (_plan.LockedCores is { } alreadyLocked) excluded.UnionWith(alreadyLocked);
        var currentMargins = _plan.InitialCoreMargins != null
            ? new Dictionary<int, int>(_plan.InitialCoreMargins)
            : new Dictionary<int, int>();

        // What the tuner has learned per core. Without this the search has no memory and
        // cannot tell "never passed" from "passed lower down and then failed". Supplied from
        // outside when a run has several phases, so the memory outlives one phase.
        var tunerState = _plan.TunerState ?? new Dictionary<int, AutoTunerCoreState>();
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
                            _plan.FloorFor(coreIndex), _plan.IsGerman,
                            (tunerState.TryGetValue(coreIndex, out var priorFail) ? priorFail : null),
                            _plan.GuardbandFor(coreIndex));
                        currentMargins[coreIndex] = res.NextMargin;
                        tunerState[coreIndex] = res.State;
                        Emit(new TestEvent.CoreAutoTuned(coreIndex, res));
                        if (res.CoreLocked)
                        {
                            excluded.Add(coreIndex);
                            _plan.LockedCores?.Add(coreIndex);
                        }
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
                            _plan.FloorFor(coreIndex), _plan.IsGerman,
                            (tunerState.TryGetValue(coreIndex, out var priorPass) ? priorPass : null),
                            _plan.GuardbandFor(coreIndex));
                        currentMargins[coreIndex] = res.NextMargin;
                        tunerState[coreIndex] = res.State;
                        Emit(new TestEvent.CoreAutoTuned(coreIndex, res));
                        if (res.CoreLocked)
                        {
                            excluded.Add(coreIndex);
                            _plan.LockedCores?.Add(coreIndex);
                        }
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
        RestoreParkedCores(currentMargins);
        Emit(new TestEvent.RunFinished(failures));
        return failures;
    }

    /// <summary>
    /// Puts every core back on the margin the run finished with.
    ///
    /// Isolation parks the untested cores at 0 for the duration of each slot, so when the run
    /// ends only the core tested last still holds its own value. Leaving it there would mean
    /// the table on screen and the processor disagree about every other core - and the user
    /// would silently lose their undervolt without a single message saying so.
    /// </summary>
    private void RestoreParkedCores(IReadOnlyDictionary<int, int> currentMargins)
    {
        if (!_plan.IsolateTestedCore || _plan.ApplyMargin is not { } apply) return;

        int restored = 0;
        foreach (var core in _cores)
        {
            if (!currentMargins.TryGetValue(core.Index, out int margin) || margin == 0) continue;
            if (apply(core.Index, margin)) restored++;
        }

        if (restored > 0)
            Emit(new TestEvent.Info(_plan.IsGerman
                ? $"{restored} geparkte Kern(e) wieder auf ihren Wert gesetzt."
                : $"Restored {restored} parked core(s) to their value."));
    }

    private async Task<List<Failure>> TestCoreAsync(
        PhysicalCore core, int iteration, List<WheaEvent> wheaHits, CancellationToken ct)
    {
        Emit(new TestEvent.CoreStarted(core.Index, iteration));

        var found = new List<Failure>();
        using var session = _engine.Start(core, _plan.Threads);

        // Micro-bursting runs on its own clock: the monitoring loop below ticks once a second
        // and the pulses are measured in hundreds of milliseconds.
        using var pulser = _plan.Transient == TransientMode.MicroBurst
            ? StartPulser(session, ct)
            : null;

        var clock = Stopwatch.StartNew();
        var lastCpu = session.CpuTime;
        var lastCheck = clock.Elapsed;
        var nextSuspend = _plan.SuspendEvery;

        // Give the engine a moment to spin up before judging how busy the core is.
        var graceUntil = TimeSpan.FromSeconds(10);

        // "auto" runs until the engine has been through its whole workload once, but never
        // past the cap: cycle detection depends on the engine's own output and must not be
        // able to hold a core indefinitely if the engine stops reporting.
        var deadline = _plan.IsAutoRuntime ? _plan.AutoRuntimeCap : _plan.RuntimePerCore;

        while (clock.Elapsed < deadline && !ct.IsCancellationRequested)
        {
            await Task.Delay(1000, ct).ConfigureAwait(false);

            found.AddRange(session.DrainFailures());

            lock (wheaHits)
            {
                foreach (var hit in wheaHits)
                {
                    if (hit.IsWarning && !_plan.TreatWheaWarningAsError) continue;

                    // A machine check names the logical processor that raised it. When that is
                    // some other core, blaming this one would back off a value that was never
                    // at fault - so it is reported, but not counted against the core under test.
                    int? blamed = hit.ApicId is { } id ? WheaWatcher.CoreForApicId(id, _cores) : null;
                    if (blamed is { } other && other != core.Index)
                    {
                        Emit(new TestEvent.Info(_plan.IsGerman
                            ? $"WHEA-Ereignis von Kern {other}, während Kern {core.Index} getestet wurde — nicht diesem Kern angelastet."
                            : $"WHEA event from core {other} while core {core.Index} was under test — not counted against this core."));
                        continue;
                    }

                    found.Add(new Failure(FailureKind.MachineCheck, hit.Description, hit.Time));
                }
                wheaHits.Clear();
            }

            // Checked after the drain, never before: a machine check raised in the same second
            // the sweep finished still belongs to this core's result.
            if (_plan.IsAutoRuntime && session.WorkloadCyclesCompleted >= 1)
            {
                Emit(new TestEvent.Info(_plan.IsGerman
                    ? $"Kern {core.Index}: kompletter Testdurchlauf nach {clock.Elapsed:h\\:mm\\:ss} abgeschlossen."
                    : $"Core {core.Index}: full workload completed after {clock.Elapsed:h\\:mm\\:ss}."));
                break;
            }

            var now = clock.Elapsed;

            if (_plan.SuspendPeriodically
                && _plan.Transient == TransientMode.Periodic
                && now >= nextSuspend)
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

                // Half the expected occupancy means the load is not where it should be — and
                // "expected" has to account for micro-bursting, which idles the core on purpose.
                double expected = _plan.Threads * _plan.ExpectedDutyCycle;
                if (wall > 0 && busy / wall < expected * 0.5)
                    found.Add(new Failure(FailureKind.WentIdle,
                        $"Core {core.Index} ran at {busy / wall:F2} of {expected:F2} expected thread(s).",
                        DateTime.Now));
            }
            lastCpu = cpu;
            lastCheck = now;

            if (found.Count > 0) break;
        }

        session.Stop();
        return found;
    }

    /// <summary>
    /// Pulses the load on and off for the length of a slot, faster than the monitoring loop
    /// ticks. Always resumes on the way out: leaving the process suspended would look exactly
    /// like a core that went idle, and the next slot would inherit a frozen engine.
    /// </summary>
    private CancellationTokenSource StartPulser(IStressSession session, CancellationToken ct)
    {
        var cts = CancellationTokenSource.CreateLinkedTokenSource(ct);
        var load = _plan.BurstLoad;
        var idle = _plan.BurstIdle;

        _ = Task.Run(async () =>
        {
            try
            {
                while (!cts.IsCancellationRequested)
                {
                    await Task.Delay(load, cts.Token).ConfigureAwait(false);
                    session.Suspend();
                    await Task.Delay(idle, cts.Token).ConfigureAwait(false);
                    session.Resume();
                }
            }
            catch (OperationCanceledException) { }
            catch (ObjectDisposedException) { }
            finally
            {
                try { session.Resume(); } catch { }
            }
        }, cts.Token);

        return cts;
    }

    /// <summary>Puts the processor at the value about to be measured, and says so out loud.</summary>
    private void ApplyPlannedMargin(int coreIndex, IReadOnlyDictionary<int, int> currentMargins)
    {
        if (_plan.ApplyMargin is not { } apply) return;
        if (!currentMargins.TryGetValue(coreIndex, out int planned)) return;

        if (_plan.IsolateTestedCore)
        {
            // Park the rest at 0 first, so the core under test is the only one that can be the
            // source of an error while its slot runs.
            int parked = 0;
            foreach (var other in _cores)
            {
                if (other.Index == coreIndex) continue;
                if (currentMargins.TryGetValue(other.Index, out int held) && held == 0) continue;
                if (apply(other.Index, 0)) parked++;
            }
            if (parked > 0)
                Emit(new TestEvent.Info(_plan.IsGerman
                    ? $"{parked} andere Kern(e) für die Dauer dieses Tests auf 0 geparkt."
                    : $"Parked {parked} other core(s) at 0 for the duration of this test."));
        }

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
            CoreOrder.CorePairs => [.. Pairs(all)],
            _ => all,
        };
    }

    /// <summary>
    /// Every ordered pair of distinct cores, walked as a flat sequence. The point is the
    /// handover: some instabilities only show when the load moves from one specific core to
    /// another, and no ordering that visits each core once can produce that transition.
    /// </summary>
    private static IEnumerable<int> Pairs(List<int> all)
    {
        foreach (int a in all)
            foreach (int b in all)
            {
                if (a == b) continue;
                yield return a;
                yield return b;
            }
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
