namespace PboStudio.Core;

/// <summary>Where a machine stands in the tuning process. Derived, never asked for.</summary>
public enum TuningStage
{
    /// <summary>No kernel driver, so Curve Optimizer values can be neither read nor written.</summary>
    InstallDriver,

    /// <summary>No stress engine, so nothing can be measured.</summary>
    InstallEngine,

    /// <summary>Nothing has been measured. The first run establishes a baseline.</summary>
    Discover,

    /// <summary>Cores still have room between what they hold and what they could try.</summary>
    Narrow,

    /// <summary>Every core is at the most aggressive value it is allowed to reach.</summary>
    Confirm,
}

/// <summary>What the card's button should do.</summary>
public enum NextStepAction
{
    /// <summary>Open the driver and engine manager.</summary>
    OpenSetup,

    /// <summary>Configure the run: profile, auto-tuner, core selection, passes.</summary>
    ConfigureRun,
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
    IReadOnlyList<int> Cores,
    NextStepAction Action = NextStepAction.ConfigureRun,

    /// <summary>
    /// Set on the setup stages, where naming a profile would be a lie: there is nothing to run
    /// yet. The card hides the profile box in that case.
    /// </summary>
    bool ShowProfile = true
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
    /// <param name="driverReady">PawnIO present, i.e. CO values can be read and written.</param>
    /// <param name="engineReady">At least one stress engine installed.</param>
    public static NextStep Recommend(
        string cpuName,
        IReadOnlyList<PhysicalCore> cores,
        IReadOnlyDictionary<int, int> currentMargins,
        IReadOnlyDictionary<int, CoreKnowledge> knowledge,
        int chipLimit,
        bool isGerman,
        bool driverReady = true,
        bool engineReady = true)
    {
        bool isX3d = cpuName.Contains("X3D", StringComparison.OrdinalIgnoreCase);

        // Setup comes first, and it comes first here rather than in a separate banner: a card
        // that says "run Heavy FFTs" next to a disabled start button is worse than no advice.
        // The driver leads because it is the only step that also needs a restart.
        if (!driverReady) return InstallDriver(isGerman);
        if (!engineReady) return InstallEngine(isGerman);

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
    /// Step one for anybody opening the program for the first time. Says what the driver is
    /// for, because "install PawnIO" means nothing to someone who has never heard of it.
    /// </summary>
    private static NextStep InstallDriver(bool isGerman) =>
        new(ProfileId.HeavyFfts, TuningStage.InstallDriver,
            isGerman ? "Schritt 1: Treiber installieren" : "Step 1: install the driver",
            isGerman
                ? "PboStudio liest und schreibt die Curve-Optimizer-Werte direkt im Prozessor. Dafür braucht Windows den PawnIO-Treiber — ohne ihn lassen sich Werte weder auslesen noch setzen, und die Live-Anzeige für Takt und Temperatur bleibt leer. Der Assistent lädt und installiert ihn; danach muss PboStudio einmal neu starten."
                : "PboStudio reads and writes Curve Optimizer values in the processor itself. Windows needs the PawnIO driver for that — without it no value can be read or set, and the live clock and temperature readouts stay empty. The assistant downloads and installs it; PboStudio then has to restart once.",
            UseAutoTuner: false,
            Cores: [],
            Action: NextStepAction.OpenSetup,
            ShowProfile: false);

    /// <summary>
    /// Step two: something has to generate the load. Names what the two engines are, since the
    /// choice looks arbitrary without that.
    /// </summary>
    private static NextStep InstallEngine(bool isGerman) =>
        new(ProfileId.HeavyFfts, TuningStage.InstallEngine,
            isGerman ? "Schritt 2: Testprogramm laden" : "Step 2: download a stress engine",
            isGerman
                ? "Die Last erzeugt ein externes Programm: Prime95 rechnet Primzahlen und belastet den Kern bei Höchsttakt, y-cruncher berechnet Pi und beansprucht Cache und Speicher ganz anders. Beide sind kostenlos, der Assistent lädt sie von den Seiten der Hersteller. Eines reicht zum Starten — beide zu haben ist besser, weil ein Kern das eine bestehen und am anderen scheitern kann."
                : "The load comes from an external program: Prime95 computes primes and stresses the core at peak clock, y-cruncher computes pi and leans on cache and memory in a completely different way. Both are free and the assistant fetches them from the vendors' own pages. One is enough to start — having both is better, because a core can pass one and fail the other.",
            UseAutoTuner: false,
            Cores: [],
            Action: NextStepAction.OpenSetup,
            ShowProfile: false);

    /// <summary>
    /// Nothing measured yet, so the first run is a plain validation of whatever is set — and
    /// the established way to validate a Curve Optimizer setting is to alternate the two
    /// engines. Prime95 SSE finds instability at peak boost and low voltage; y-cruncher leans
    /// on cache and memory in a way Prime95's FFT work does not, and catches cores that sail
    /// through it. CoreCycler needs two runs and its multiconfig launcher to do this; the
    /// combined profile chains both in one.
    /// </summary>
    private static NextStep Discover(string cpuName, bool isX3d, bool isGerman) =>
        new(ProfileId.RecommendedCombo, TuningStage.Discover,
            isGerman ? "Schritt 3: den ersten Testlauf machen" : "Step 3: make the first run",
            isGerman
                ? $"Für den {cpuName} ist noch nichts gemessen. „Heavy FFTs“ ist der Standardeinstieg: SSE deckt instabile Kerne am schnellsten auf und erzeugt dabei die geringste Hitze{(isX3d ? " — bei einem X3D wichtig, weil der gestapelte Cache empfindlich auf Temperatur reagiert" : "")}.\n\nDieser Lauf verändert nichts dauerhaft: er belastet jeden Kern nacheinander mit den Werten, die gerade anliegen, und hält fest, welcher durchfällt. Erst danach wird über neue Werte entschieden — und auch die gelten nur bis zum nächsten Neustart, solange du sie nicht ins BIOS überträgst."
                : $"Nothing has been measured on the {cpuName} yet. “Heavy FFTs” is the standard opener: SSE exposes unstable cores fastest and produces the least heat doing it{(isX3d ? " — which matters on an X3D, whose stacked cache is sensitive to temperature" : "")}.\n\nThis run changes nothing permanently: it loads each core in turn at whatever values are currently set and records which ones fail. New values come after that — and even those only last until the next reboot unless you carry them into the BIOS.",
            UseAutoTuner: false,
            Cores: []);

    /// <summary>
    /// Some cores hold a milder value than they are allowed to try. That is a search, which is
    /// what the auto-tuner is for — and running it on only the open cores is the difference
    /// between an evening and a night.
    /// <para>
    /// Single engine on purpose, unlike the runs either side of it. Locked cores are shared
    /// across phases and excluded at the start of each, so a core the tuner settles during the
    /// Prime95 phase never enters the y-cruncher phase at all: the second engine would cost the
    /// full runtime and test nothing. Both load types belong on the confirmation run, where
    /// nothing is locked and every core is measured under all of them.
    /// </para>
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
                  + "Der Auto-Tuner senkt schrittweise ab und fixiert den ersten Wert, der hält; Werte, bei denen ein Kern schon einmal durchgefallen ist, fasst er nicht mehr an.\n\n"
                  + "Hier läuft bewusst nur Prime95, nicht der Wechsel mit y-cruncher: ein Kern, der in der ersten Phase fixiert wird, ist aus der zweiten ausgeschlossen und bekäme y-cruncher gar nicht mehr zu sehen. Die zweite Lastart kommt beim Absichern danach."
                : $"{(open.Count == 1 ? "This core holds" : "These cores hold")} a milder value than what is currently known to be reachable. "
                  + $"{(few ? "The rest are maxed out and stay out of the run, which saves most of the runtime. " : "")}"
                  + "The auto-tuner steps down and locks the first value that holds; values a core has already failed at are off limits.\n\n"
                  + "This stage deliberately runs Prime95 alone rather than alternating with y-cruncher: a core locked in the first phase is excluded from the second and would never see y-cruncher at all. The second load type comes with the confirmation run afterwards.",
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
                ? "Jeder Kern steht auf dem aggressivsten Wert, der nach bisherigem Stand zulässig ist. Der Absicherungslauf fährt SSE über alle FFT-Größen, danach AVX, AVX2 und y-cruncher — vier Lastarten, beide Testprogramme. Erst das rechtfertigt, die Werte ins BIOS zu übertragen."
                + (isX3d ? " Behalte die Temperatur im Auge: der AVX2-Abschnitt ist auf einem X3D der heißeste Teil des Laufs." : "")
                : "Every core sits at the most aggressive value currently known to be allowed. The overnight run sweeps SSE across every FFT size, then AVX, AVX2 and y-cruncher — four load types across both engines. Only that justifies committing the values to the BIOS."
                + (isX3d ? " Watch the temperature: on an X3D the AVX2 leg is the hottest part of the run." : ""),
            UseAutoTuner: false,
            Cores: []);
}
