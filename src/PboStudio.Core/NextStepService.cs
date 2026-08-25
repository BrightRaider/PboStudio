namespace PboStudio.Core;

/// <summary>Where a machine stands in the tuning process. Derived, never asked for.</summary>
public enum TuningStage
{
    /// <summary>Nothing has been measured. The first run establishes a baseline.</summary>
    Discover,

    /// <summary>Cores still have room between what they hold and what they could try.</summary>
    Narrow,

    /// <summary>Every core is at the most aggressive value it is allowed to reach.</summary>
    Confirm,
}

/// <param name="Profile">Which profile to run. An id, so nothing has to match by name.</param>
/// <param name="Headline">One line naming what to do.</param>
/// <param name="Reason">Why, in terms of this machine rather than in general.</param>
/// <param name="UseAutoTuner">Whether the auto-tuner should drive this run.</param>
/// <param name="Cores">Cores worth testing. Empty means all of them.</param>
public sealed record NextStep(
    ProfileId Profile,
    TuningStage Stage,
    string Headline,
    string Reason,
    bool UseAutoTuner,
    IReadOnlyList<int> Cores
);

/// <summary>
/// The single recommendation the interface leads with.
///
/// Thirteen profiles are thirteen answers to a question nobody asked. What a person actually
/// wants to know is "what should I run now", and that has one correct answer at any moment —
/// it just depends on the processor and on what has already been established, which the
/// program knows and the reader should not have to work out.
///
/// This replaces a lookup table that mapped CPU generation to a profile *name*, which the UI
/// then matched back by counting shared words. The names drifted apart, and for an X3D part it
/// recommended the AVX2 profile: the hottest load in the set, on the one design whose stacked
/// cache is most sensitive to temperature.
/// </summary>
public static class NextStepService
{
    public static NextStep Recommend(
        string cpuName,
        IReadOnlyList<PhysicalCore> cores,
        IReadOnlyDictionary<int, int> currentMargins,
        IReadOnlyDictionary<int, CoreKnowledge> knowledge,
        int chipLimit,
        bool isGerman)
    {
        bool isX3d = cpuName.Contains("X3D", StringComparison.OrdinalIgnoreCase);

        // A core has room left when it holds a value milder than the floor it is still allowed
        // to try. The floor comes from what this core has already been observed to do, so a
        // core that crashed at -30 counts as finished at -29 rather than being chased further.
        var open = new List<int>();
        foreach (var core in cores)
        {
            int held = currentMargins.TryGetValue(core.Index, out int m) ? m : 0;
            int floor = knowledge.TryGetValue(core.Index, out var k)
                ? k.FloorFor(chipLimit)
                : chipLimit;

            if (held > floor) open.Add(core.Index);
        }

        bool nothingKnown = knowledge.Count == 0
            || cores.All(c => !knowledge.TryGetValue(c.Index, out var k) || k.BestPassed is null);

        if (nothingKnown)
            return Discover(cpuName, isX3d, isGerman);

        if (open.Count > 0)
            return Narrow(open, cores.Count, isGerman);

        return Confirm(isX3d, isGerman);
    }

    /// <summary>
    /// Nothing measured yet. SSE across the Heavy range is the workhorse: it exposes an
    /// undervolted core fastest, and because it is the coolest of the Prime95 modes it is the
    /// right first load on every part, X3D included.
    /// </summary>
    private static NextStep Discover(string cpuName, bool isX3d, bool isGerman) =>
        new(ProfileId.HeavyFfts, TuningStage.Discover,
            isGerman ? "Erst einmal herausfinden, wo du stehst" : "First, find out where you stand",
            isGerman
                ? $"Für den {cpuName} ist noch nichts gemessen. „Heavy FFTs“ ist der Standardeinstieg: SSE deckt instabile Kerne am schnellsten auf und erzeugt dabei die geringste Hitze{(isX3d ? " — bei einem X3D ist genau das wichtig, weil der gestapelte Cache empfindlich auf Temperatur reagiert" : "")}."
                : $"Nothing has been measured on the {cpuName} yet. “Heavy FFTs” is the standard opener: SSE exposes unstable cores fastest and produces the least heat doing it{(isX3d ? " — which matters on an X3D, whose stacked cache is sensitive to temperature" : "")}.",
            UseAutoTuner: false,
            Cores: []);

    /// <summary>
    /// Some cores hold a milder value than they are allowed to try. That is a search, which is
    /// what the auto-tuner is for — and running it on only the open cores is the difference
    /// between an evening and a night.
    /// </summary>
    private static NextStep Narrow(IReadOnlyList<int> open, int total, bool isGerman)
    {
        string list = string.Join(", ", open);
        bool few = open.Count < total;

        return new NextStep(ProfileId.HeavyFfts, TuningStage.Narrow,
            isGerman
                ? $"Auto-Tuner auf {(open.Count == 1 ? $"Kern {list}" : $"die Kerne {list}")} ansetzen"
                : $"Point the auto-tuner at {(open.Count == 1 ? $"core {list}" : $"cores {list}")}",
            isGerman
                ? $"{(open.Count == 1 ? "Dieser Kern hält" : "Diese Kerne halten")} noch einen milderen Wert, als nach bisherigem Stand möglich wäre. "
                  + $"{(few ? "Die übrigen Kerne sind ausgereizt und bleiben außen vor — das spart den Großteil der Laufzeit. " : "")}"
                  + "Der Auto-Tuner senkt schrittweise ab und fixiert den ersten Wert, der hält; Werte, bei denen ein Kern schon einmal durchgefallen ist, fasst er nicht mehr an."
                : $"{(open.Count == 1 ? "This core holds" : "These cores hold")} a milder value than what is currently known to be reachable. "
                  + $"{(few ? "The rest are maxed out and stay out of the run, which saves most of the runtime. " : "")}"
                  + "The auto-tuner steps down and locks the first value that holds; values a core has already failed at are off limits.",
            UseAutoTuner: true,
            Cores: open);
    }

    /// <summary>
    /// Nothing left to search. What remains is proving the result under a load type it has not
    /// seen, which is what the overnight run chains together.
    /// </summary>
    private static NextStep Confirm(bool isX3d, bool isGerman) =>
        new(ProfileId.Overnight, TuningStage.Confirm,
            isGerman ? "Jetzt absichern" : "Now confirm it",
            isGerman
                ? "Jeder Kern steht auf dem aggressivsten Wert, der nach bisherigem Stand zulässig ist. Der Absicherungslauf fährt SSE über alle FFT-Größen, danach AVX, AVX2 und y-cruncher — vier verschiedene Lastarten. Erst das rechtfertigt, die Werte ins BIOS zu übertragen."
                + (isX3d ? " Behalte die Temperatur im Auge: der AVX2-Abschnitt ist auf einem X3D der heißeste Teil des Laufs." : "")
                : "Every core sits at the most aggressive value currently known to be allowed. The overnight run sweeps SSE across every FFT size, then AVX, AVX2 and y-cruncher — four different load types. Only that justifies committing the values to the BIOS."
                + (isX3d ? " Watch the temperature: on an X3D the AVX2 leg is the hottest part of the run." : ""),
            UseAutoTuner: false,
            Cores: []);
}
