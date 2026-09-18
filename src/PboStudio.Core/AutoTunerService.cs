namespace PboStudio.Core;

public enum AutoTunerMode
{
    /// <summary>Step size -3 per pass. Fast discovery of core limits.</summary>
    Grob,
    /// <summary>Step size -1 per pass. Maximum precision tuning to squeeze out every millivolt.</summary>
    Fein
}

/// <summary>
/// What the tuner has learned about one core so far, carried between passes.
/// </summary>
/// <param name="LastKnownGood">
/// The lowest margin this core has actually passed a full test slot at, if any. A value that
/// has survived a slot needs no retest, so a later failure can settle here immediately.
/// </param>
/// <param name="Ascending">
/// True once the core has failed without a known-good value, i.e. the search is now working
/// its way back up toward stability rather than down toward the limit.
/// </param>
public sealed record AutoTunerCoreState(int? LastKnownGood = null, bool Ascending = false);

public sealed record AutoTunerStepResult(
    int CoreIndex,
    int OldMargin,
    int NextMargin,
    bool CoreLocked,
    string Advice,
    AutoTunerCoreState State,
    /// <summary>Whether the core held up at <see cref="OldMargin"/>. Carried so the UI can
    /// draw the search path without re-deriving it from the direction of the next step.</summary>
    bool Passed = false
);

/// <summary>
/// Auto-Tuner engine. Walks each core down toward the chip limit, and on failure walks it back
/// up until it passes.
/// <para>
/// The upward half is the point. Backing off by a fixed amount and declaring the core done
/// produced a value nobody had ever tested - a guess wearing the label "locked". A core that
/// fails at its very first margin now keeps getting retested at progressively safer values
/// until one of them actually holds.
/// </para>
/// </summary>
public static class AutoTunerService
{
    public const int DefaultBiosLimit = -30;

    /// <summary>
    /// The most negative Curve Optimizer offset this processor accepts.
    /// <para>
    /// This used to return -50 for the Ryzen 9000 series, on the reading that Zen 5 has an
    /// extended range. It does not: Curve Optimizer is -30 to +30 on every AMD desktop part
    /// that has it. What Zen 5 added is Curve Shaper, a second and separate set of offsets
    /// across temperature and frequency bands, which this program does not write. A search that
    /// ran to -50 would end by reporting values nobody can type into a BIOS.
    /// </para>
    /// </summary>
    public static int GetMaxNegativeMargin(string cpuName) =>
        CpuModelService.Detect(cpuName) is { SupportsCurveOptimizer: true } cpu
            ? cpu.MaxNegativeMargin
            : DefaultBiosLimit;

    /// <summary>Symmetric: searching up in bigger jumps than it came down would skip candidates.</summary>
    public static int StepSize(AutoTunerMode mode) => mode == AutoTunerMode.Grob ? 3 : 1;

    /// <summary>
    /// Points of headroom left between the most aggressive value a core survived and the value
    /// it is actually locked at.
    ///
    /// A stress test finds the boundary margin — the value at which the core just barely holds
    /// under that particular load. Everyday use is not that load: temperature drifts, other
    /// cores come and go, and a light-load game swings the core to its highest boost clock,
    /// where a negative offset has the least voltage to give. Locking the boundary itself hands
    /// the user a value with nothing left for any of that, and it fails in a way the test never
    /// reproduces — a hard reboot mid-game with WHEA 18 in the event log.
    /// </summary>
    public const int DefaultGuardband = 3;

    /// <summary>
    /// Extra headroom for the cores CPPC ranks highest. They boost furthest, and Windows puts
    /// its background work on them, so they spend the most time in exactly the state that
    /// exposes an over-aggressive curve.
    /// </summary>
    public const int PreferredCoreExtraGuardband = 2;

    /// <summary>Where a core is locked, given what it actually survived.</summary>
    public static int ApplyGuardband(int lastKnownGood, int guardband) =>
        Math.Min(0, lastKnownGood + Math.Max(0, guardband));

    public static AutoTunerStepResult CalculateNextStep(
        int coreIndex,
        int currentMargin,
        bool passed,
        AutoTunerMode mode,
        int maxLimit = DefaultBiosLimit,
        bool isGerman = false,
        AutoTunerCoreState? state = null,
        int guardband = 0)
    {
        state ??= new AutoTunerCoreState();
        int step = StepSize(mode);

        var result = passed
            ? OnPassed(coreIndex, currentMargin, mode, maxLimit, isGerman, state, step, guardband)
            : OnFailed(coreIndex, currentMargin, isGerman, state, step, guardband);

        return result with { Passed = passed };
    }

    /// <summary>
    /// Names the headroom in the advice line, so a locked value that differs from the one that
    /// was measured never looks like a mistake.
    /// </summary>
    private static string GuardNote(int tested, int locked, bool isGerman)
    {
        int gap = locked - tested;
        if (gap <= 0) return " (🔒)";

        return isGerman
            ? $" (🔒) — {gap} Stufen Sicherheitsabstand auf den gemessenen Grenzwert {tested}"
            : $" (🔒) — {gap} points of headroom over the measured boundary of {tested}";
    }

    private static AutoTunerStepResult OnPassed(
        int coreIndex, int currentMargin, AutoTunerMode mode, int maxLimit,
        bool isGerman, AutoTunerCoreState state, int step, int guardband)
    {
        var learned = state with { LastKnownGood = currentMargin };

        // Coming back up after a failure: this is the first value that holds, and everything
        // below it is known bad. There is nothing left to look for.
        if (state.Ascending)
        {
            int safe = ApplyGuardband(currentMargin, guardband);
            return new AutoTunerStepResult(
                coreIndex, currentMargin, safe, CoreLocked: true,
                isGerman
                    ? $"🎯 Kern {coreIndex}: stabil bei {currentMargin} — tiefere Werte sind durchgefallen. Fixiert bei {safe}{GuardNote(currentMargin, safe, isGerman)}."
                    : $"🎯 Core {coreIndex}: stable at {currentMargin} — lower values failed. Locked at {safe}{GuardNote(currentMargin, safe, isGerman)}.",
                learned);
        }

        if (currentMargin <= maxLimit)
        {
            int safe = ApplyGuardband(currentMargin, guardband);
            return new AutoTunerStepResult(
                coreIndex, currentMargin, safe, CoreLocked: true,
                isGerman
                    ? $"🎯 Kern {coreIndex}: Limit ({maxLimit}) bestanden, fixiert bei {safe}{GuardNote(currentMargin, safe, isGerman)}."
                    : $"🎯 Core {coreIndex}: passed at the limit ({maxLimit}), locked at {safe}{GuardNote(currentMargin, safe, isGerman)}.",
                learned);
        }

        int next = Math.Max(maxLimit, currentMargin - step);
        return new AutoTunerStepResult(
            coreIndex, currentMargin, next, CoreLocked: false,
            isGerman
                ? $"✅ Kern {coreIndex} bestanden bei {currentMargin}. Nächste Stufe: {next}"
                : $"✅ Core {coreIndex} passed at {currentMargin}. Next step: {next}",
            learned);
    }

    private static AutoTunerStepResult OnFailed(
        int coreIndex, int currentMargin, bool isGerman, AutoTunerCoreState state, int step, int guardband)
    {
        // A value this core already survived is proven, so there is no reason to test it again.
        if (state.LastKnownGood is { } good)
        {
            int safe = ApplyGuardband(good, guardband);
            return new AutoTunerStepResult(
                coreIndex, currentMargin, safe, CoreLocked: true,
                isGerman
                    ? $"⚠️ Kern {coreIndex} instabil bei {currentMargin}. Zuletzt bestanden: {good}. Fixiert bei {safe}{GuardNote(good, safe, isGerman)}."
                    : $"⚠️ Core {coreIndex} unstable at {currentMargin}. Last passed at {good}. Locked at {safe}{GuardNote(good, safe, isGerman)}.",
                state);
        }

        // Nothing has held yet. Failing without undervolt means the cause is not the curve.
        if (currentMargin >= 0)
        {
            return new AutoTunerStepResult(
                coreIndex, currentMargin, 0, CoreLocked: true,
                isGerman
                    ? $"⛔ Kern {coreIndex} fällt selbst ohne Undervolt (0) durch. Die Ursache liegt nicht am Curve Optimizer — prüfe RAM/EXPO, Kühlung und BIOS-Version."
                    : $"⛔ Core {coreIndex} fails even without undervolt (0). The cause is not the Curve Optimizer — check RAM/EXPO, cooling and BIOS version.",
                state with { Ascending = true });
        }

        int next = Math.Min(0, currentMargin + step);
        return new AutoTunerStepResult(
            coreIndex, currentMargin, next, CoreLocked: false,
            isGerman
                ? $"⚠️ Kern {coreIndex} instabil bei {currentMargin}. Spannung erhöht auf {next} und erneut testen."
                : $"⚠️ Core {coreIndex} unstable at {currentMargin}. Raising voltage to {next} and testing again.",
            state with { Ascending = true });
    }

    /// <summary>
    /// Worst-case number of test slots one core can consume, for the runtime estimate.
    /// <para>
    /// The two directions are alternatives, not a sum. A core that descends only ever climbs
    /// back to a value it already passed, which costs no further slot; a core climbs from the
    /// start only if its very first test fails. So the worst case is whichever leg is longer.
    /// </para>
    /// </summary>
    /// <param name="canClimb">
    /// Whether the search might have to walk this core back up from scratch. False once the core
    /// has a value on record that held: a failure then returns straight to that value and locks
    /// there, without retesting it, so the climb cannot happen.
    /// <para>
    /// Leaving this out overstated the estimate badly for exactly the cores a campaign is
    /// usually left with. A core sitting at -25 with -25 on record as passing has four steps to
    /// try and then it is done — five slots. Costed as though it might climb from -25 back to 0,
    /// it came out at twenty-six, and a half-hour phase was quoted at two and a half hours.
    /// </para>
    /// </param>
    public static int WorstCaseSlots(
        int currentMargin, int maxLimit, AutoTunerMode mode, int maxPasses, bool canClimb = true)
    {
        int step = StepSize(mode);

        int descentSlots = currentMargin > maxLimit
            ? (int)Math.Ceiling((double)(currentMargin - maxLimit) / step) + 1
            : 1;

        int ascentSlots = canClimb
            ? (int)Math.Ceiling((double)Math.Max(0, -currentMargin) / step) + 1
            : 0;

        return Math.Max(1, Math.Min(maxPasses, Math.Max(descentSlots, ascentSlots)));
    }
}
