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
    AutoTunerCoreState State
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
    public const int ExtendedZen5Limit = -50;

    /// <summary>Detects hardware/BIOS maximum negative Curve Optimizer limit based on CPU architecture.</summary>
    public static int GetMaxNegativeMargin(string cpuName)
    {
        if (string.IsNullOrEmpty(cpuName)) return DefaultBiosLimit;

        // Zen 5 (Ryzen 9000 series) supports extended Curve Shaper / SMU range down to -50
        if (cpuName.Contains("9950") || cpuName.Contains("9900") || cpuName.Contains("9800") ||
            cpuName.Contains("9700") || cpuName.Contains("9600"))
        {
            return ExtendedZen5Limit;
        }

        // Zen 2, Zen 3 (5000 series), Zen 4 (7000/8000 series) use standard -30 limit
        return DefaultBiosLimit;
    }

    /// <summary>Symmetric: searching up in bigger jumps than it came down would skip candidates.</summary>
    public static int StepSize(AutoTunerMode mode) => mode == AutoTunerMode.Grob ? 3 : 1;

    public static AutoTunerStepResult CalculateNextStep(
        int coreIndex,
        int currentMargin,
        bool passed,
        AutoTunerMode mode,
        int maxLimit = DefaultBiosLimit,
        bool isGerman = false,
        AutoTunerCoreState? state = null)
    {
        state ??= new AutoTunerCoreState();
        int step = StepSize(mode);

        return passed
            ? OnPassed(coreIndex, currentMargin, mode, maxLimit, isGerman, state, step)
            : OnFailed(coreIndex, currentMargin, isGerman, state, step);
    }

    private static AutoTunerStepResult OnPassed(
        int coreIndex, int currentMargin, AutoTunerMode mode, int maxLimit,
        bool isGerman, AutoTunerCoreState state, int step)
    {
        var learned = state with { LastKnownGood = currentMargin };

        // Coming back up after a failure: this is the first value that holds, and everything
        // below it is known bad. There is nothing left to look for.
        if (state.Ascending)
        {
            return new AutoTunerStepResult(
                coreIndex, currentMargin, currentMargin, CoreLocked: true,
                isGerman
                    ? $"🎯 Kern {coreIndex}: stabil bei {currentMargin} — tiefere Werte sind durchgefallen. Fixiert (🔒)."
                    : $"🎯 Core {coreIndex}: stable at {currentMargin} — lower values failed. Locked (🔒).",
                learned);
        }

        if (currentMargin <= maxLimit)
        {
            return new AutoTunerStepResult(
                coreIndex, currentMargin, currentMargin, CoreLocked: true,
                isGerman
                    ? $"🎯 Kern {coreIndex}: CPU-Limit ({maxLimit}) bestanden und fixiert (🔒)."
                    : $"🎯 Core {coreIndex}: passed at the CPU limit ({maxLimit}) and locked (🔒).",
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
        int coreIndex, int currentMargin, bool isGerman, AutoTunerCoreState state, int step)
    {
        // A value this core already survived is proven, so there is no reason to test it again.
        if (state.LastKnownGood is { } good)
        {
            return new AutoTunerStepResult(
                coreIndex, currentMargin, good, CoreLocked: true,
                isGerman
                    ? $"⚠️ Kern {coreIndex} instabil bei {currentMargin}. Zurück auf den zuletzt bestandenen Wert {good} und fixiert (🔒)."
                    : $"⚠️ Core {coreIndex} unstable at {currentMargin}. Back to the last value it passed, {good}, and locked (🔒).",
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
    public static int WorstCaseSlots(int currentMargin, int maxLimit, AutoTunerMode mode, int maxPasses)
    {
        int step = StepSize(mode);

        int descentSlots = currentMargin > maxLimit
            ? (int)Math.Ceiling((double)(currentMargin - maxLimit) / step) + 1
            : 1;

        int ascentSlots = (int)Math.Ceiling((double)Math.Max(0, -currentMargin) / step) + 1;

        return Math.Max(1, Math.Min(maxPasses, Math.Max(descentSlots, ascentSlots)));
    }
}
