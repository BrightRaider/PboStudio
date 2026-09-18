namespace PboStudio.Core;

/// <summary>One leg of a run: an engine, instruction set, and FFT range or algorithm preset.</summary>
public sealed record TestPhase(
    Prime95Mode Mode = Prime95Mode.Sse,
    FftPreset Fft = FftPreset.Huge,
    EngineKind Engine = EngineKind.Prime95,
    YCruncherOptions? YcOptions = null)
{
    public string Label => Engine == EngineKind.YCruncher
        ? "y-cruncher"
        : $"{Mode.ToString().ToUpperInvariant()} / {Fft}";
}

/// <summary>
/// Stable identity for a profile. Recommendations used to name their target as a display
/// string and the UI matched it back by counting shared words, because the two lists were
/// maintained separately and their wording drifted apart. An id cannot drift.
/// </summary>
public enum ProfileId
{
    RecommendedCombo,
    HeavyFfts,
    FullPrime95,
    QuickCheck,
    BreakingPoint,
    HeavyLoad,
    YCruncherCo,
    YCruncherAll,
    Step1Sse,
    Step2Avx2,
    SmallFfts,
    Overnight,
    MemoryFabric,
    MicroBurst,
}

public sealed record TestProfile(
    string Name,
    string Explanation,
    Prime95Mode Mode,
    FftPreset Fft,
    int Threads,
    int MinutesPerCore,
    int Iterations = 3,
    CoreOrder Order = CoreOrder.Alternate,
    IReadOnlyList<TestPhase>? ExtraPhases = null,
    EngineKind Engine = EngineKind.Prime95,
    YCruncherOptions? YcOptions = null,
    /// <summary>
    /// Transient pause, in seconds. Interrupting the load so the core drops to idle and boosts
    /// back up is what breaks a borderline Curve Optimizer value; a steady load often will not.
    /// Carried by the profile so picking one is genuinely a single decision.
    /// </summary>
    int SuspendEverySeconds = 30,
    int SuspendForSeconds = 1)
{
    /// <summary>
    /// Which shelf this profile belongs on. The grouping existed only as source comments, so
    /// the UI showed thirteen equally-weighted entries and left a newcomer to guess whether
    /// "Heavy FFTs" comes before or after "Step 1 - SSE".
    /// </summary>
    public string Category { get; init; } = "";

    public ProfileId Id { get; init; } = ProfileId.HeavyFfts;

    /// <summary>
    /// Whether this profile appears in the short list.
    ///
    /// Fourteen entries is not a choice, it is a quiz: nine of them are variations on the five
    /// that actually get somebody from an untested processor to values worth putting in the
    /// BIOS. The five are the path — first run, narrowing, a short look, the case a stress test
    /// cannot produce, and the final proof. The rest stay one checkbox away.
    /// </summary>
    public bool Essential { get; init; }

    /// <summary>
    /// How this profile interrupts the load. Micro-bursting pulses far below the one-second
    /// granularity <see cref="SuspendEverySeconds"/> can express, so it needs its own flag.
    /// </summary>
    public TransientMode Transient { get; init; } = TransientMode.Periodic;

    /// <summary>
    /// Every leg this profile runs, in order. Most profiles are a single leg; the combined ones
    /// chain several so nobody has to reconfigure and restart between instruction sets or engines.
    /// </summary>
    public IReadOnlyList<TestPhase> Phases =>
        [new TestPhase(Mode, Fft, Engine, YcOptions), .. ExtraPhases ?? []];

    /// <summary>Which engines this profile drives, for the "what does it do" line in the UI.</summary>
    public IReadOnlyList<EngineKind> Engines =>
        [.. Phases.Select(p => p.Engine).Distinct()];

    public bool Uses(EngineKind kind) => Engines.Contains(kind);

    /// <summary>
    /// The engine and workload summary shown under the profile name, so the separate engine
    /// selector is unnecessary: the profile states what it runs.
    /// </summary>
    public string EngineSummary
    {
        get
        {
            var parts = Phases.Select(p => p.Engine == EngineKind.YCruncher
                ? "y-cruncher"
                : $"Prime95 {p.Mode.ToString().ToUpperInvariant()} · {p.Fft}");

            string chain = string.Join("  →  ", parts);

            string runtime = LocalizationService.Pick(
                $"{MinutesPerCore} Min/Kern × {Iterations}",
                $"{MinutesPerCore} min/core × {Iterations}");

            string threads = Threads > 1
                ? LocalizationService.Pick($", {Threads} Threads", $", {Threads} threads")
                : "";

            string pause = SuspendEverySeconds > 0
                ? LocalizationService.Pick(
                    $", Lastwechsel alle {SuspendEverySeconds}s",
                    $", load transition every {SuspendEverySeconds}s")
                : "";

            return $"{chain}  ·  {runtime}{threads}{pause}";
        }
    }

    public override string ToString() => Name;
}

/// <summary>
/// Ready-made settings, so nobody has to guess which knobs matter. Each profile carries its
/// engine, workload, runtime and transient-pause behaviour: selecting one is the whole
/// configuration, which is the point of having profiles at all.
/// </summary>
public static class TestProfiles
{
    public static IReadOnlyList<TestProfile> For(string cpuName, bool isGerman = false)
    {
        bool isX3d = cpuName.Contains("X3D", StringComparison.OrdinalIgnoreCase);

        string catEveryday  = isGerman ? "FÜR DEN ALLTAG"      : "EVERYDAY";
        string catLimit     = isGerman ? "GRENZWERT FINDEN"    : "FINDING THE LIMIT";
        string catYc        = isGerman ? "NUR Y-CRUNCHER"      : "Y-CRUNCHER ONLY";
        string catSteps     = isGerman ? "SCHRITT FÜR SCHRITT" : "STEP BY STEP";
        string catLong      = isGerman ? "DIE LANGEN"          : "THE LONG ONES";

        var list = new List<TestProfile>
        {
            // ─────────────────────────── everyday ───────────────────────────
            new(isGerman ? "🏆 Empfohlen – Prime95 SSE & y-cruncher" : "🏆 Recommended - Prime95 SSE & y-cruncher",
                isGerman
                    ? "Der Gold-Standard aus CoreCycler: Prime95 SSE für Höchsttakt und Vdroop-Grenzen, danach y-cruncher für schwere Vektor- und Cache-Last. Zwei verschiedene Lastarten zu bestehen sagt deutlich mehr aus als eine."
                    : "The gold standard from CoreCycler: Prime95 SSE for peak boost and Vdroop limits, then y-cruncher for heavy vector and cache load. Passing two different load types says considerably more than passing one.",
                Prime95Mode.Sse, FftPreset.Huge, 1, 6, 3,
                CoreOrder.Alternate,
                [new TestPhase(Prime95Mode.Avx2, FftPreset.Smallest, EngineKind.YCruncher, YCruncherOptions.CurveOptimizer)]),

            new(isGerman ? "Heavy FFTs – der CoreCycler-Klassiker" : "Heavy FFTs - the CoreCycler classic",
                isGerman
                    ? "SSE über den Bereich 4–1344K. Deckt in der Praxis die meisten instabilen Kerne am schnellsten auf und ist das, womit die meisten CoreCycler-Nutzer arbeiten."
                    : "SSE across 4-1344K. In practice this exposes most unstable cores fastest, and it is what most CoreCycler users run.",
                Prime95Mode.Sse, FftPreset.Heavy, 1, 6, 3),

            new(isGerman ? "Komplett – Prime95 SSE & AVX2" : "Full Run - Prime95 SSE & AVX2",
                isGerman
                    ? "Beide nötigen Modi nacheinander: erst SSE mit großen FFTs für Grenzfälle bei Höchsttakt, dann AVX2 mit kleinen FFTs für Stromlast. Nur Prime95, kein y-cruncher."
                    : "Both essential modes in sequence: SSE with large FFTs for peak-boost edge cases, then AVX2 with small FFTs for current load. Prime95 only, no y-cruncher.",
                Prime95Mode.Sse, FftPreset.Huge, 1, 6, 3,
                CoreOrder.Alternate,
                [new TestPhase(Prime95Mode.Avx2, FftPreset.Smallest)]),

            new(isGerman ? "Schnellcheck (3 Min / Kern)" : "Quick Check (3 min / core)",
                isGerman
                    ? "Ein Durchgang SSE, 3 Minuten pro Kern. Findet in einer halben Stunde die groben Ausreißer — nicht mehr, aber auch nicht weniger."
                    : "One SSE pass, 3 minutes per core. Finds the obvious outliers in half an hour - no more, but no less.",
                Prime95Mode.Sse, FftPreset.Huge, 1, 3, 1),

            // ─────────────────────────── finding the limit ───────────────────────────
            new(isGerman ? "🔥 Grenzwert – harte Lastwechsel" : "🔥 Breaking point - hard load transitions",
                isGerman
                    ? "Das Härteste, was das Werkzeug kann: AVX2 mit kleinsten FFTs auf beiden Threads, und alle 5 Sekunden ein Lastwechsel. Genau dieser Sprung von Leerlauf auf Volllast bringt grenzwertige CO-Werte zum Absturz, während sie Dauerlast stundenlang überleben. Nimm das, wenn ein Kern zu gut erscheint."
                    : "The hardest thing this tool does: AVX2 with the smallest FFTs on both threads, and a load transition every 5 seconds. That jump from idle to full load is what breaks borderline CO values, which often survive hours of steady load. Use it when a core looks too good.",
                Prime95Mode.Avx2, FftPreset.Smallest, 2, 10, 1,
                CoreOrder.Sequential,
                SuspendEverySeconds: 5, SuspendForSeconds: 1),

            new(isGerman ? "Volllast – Kühlung & PBO-Limits" : "Heavy Load - cooling & PBO limits",
                isGerman
                    ? "AVX2 auf beiden Threads, ohne Lastwechsel. Prüft, ob Kühlung und die PPT/TDC/EDC-Grenzen überhaupt ausreichen — eher ein Test der Plattform als der CO-Werte."
                    : "AVX2 on both threads, no load transitions. Tests whether cooling and the PPT/TDC/EDC limits hold up at all - more a test of the platform than of the CO values.",
                Prime95Mode.Avx2, FftPreset.Smallest, 2, 10, 1, CoreOrder.Sequential),

            // ─────────────────────────── y-cruncher only ───────────────────────────
            new(isGerman ? "y-cruncher – Curve Optimizer" : "y-cruncher - Curve Optimizer",
                isGerman
                    ? "Nur y-cruncher, mit den fünf Algorithmen, die erfahrungsgemäß am zuverlässigsten instabile Kerne aufdecken. Ganz anderes Lastprofil als Prime95: viel Cache- und Speicherdruck."
                    : "y-cruncher alone, with the five algorithms that most reliably expose unstable cores. A completely different load profile from Prime95: heavy cache and memory pressure.",
                Prime95Mode.Sse, FftPreset.Smallest, 1, 6, 3,
                CoreOrder.Alternate, null,
                EngineKind.YCruncher, YCruncherOptions.CurveOptimizer),

            new(isGerman ? "y-cruncher – alle 8 Algorithmen" : "y-cruncher - all 8 algorithms",
                isGerman
                    ? "Der vollständige y-cruncher-Durchlauf. Deutlich länger, dafür die breiteste Abdeckung an Rechenarten."
                    : "The complete y-cruncher run. Considerably longer, but the broadest coverage of computation types.",
                Prime95Mode.Sse, FftPreset.Smallest, 1, 10, 2,
                CoreOrder.Alternate, null,
                EngineKind.YCruncher, YCruncherOptions.Default),

            // ─────────────────────────── step by step ───────────────────────────
            new(isGerman ? "Schritt 1 – SSE (fängt hier an)" : "Step 1 - SSE (start here)",
                isGerman
                    ? "SSE mit riesigen FFTs, ein Thread. Lässt den Kern bei geringer Spannung maximal hoch boosten — der klassische erste Schritt."
                    : "SSE with huge FFTs, one thread. Lets the core boost as high as it can at low voltage - the classic first step.",
                Prime95Mode.Sse, FftPreset.Huge, 1, 6, 3),

            new(isGerman ? "Schritt 2 – AVX2 (danach)" : "Step 2 - AVX2 (after that)",
                isGerman
                    ? "Dieselbe Runde mit AVX2. Beansprucht Schaltungen, die SSE nie anfasst, und zieht deutlich mehr Strom."
                    : "The same round with AVX2. Stresses execution units SSE never touches, and draws considerably more current.",
                Prime95Mode.Avx2, FftPreset.Smallest, 1, 6, 3),

            new(isGerman ? "Kleine FFTs (Schnellsuche)" : "Small FFTs (fast discovery)",
                isGerman
                    ? "Cache-interne FFTs, ein Thread. Deckt anfängliche Fehler in der Praxis am schnellsten auf."
                    : "In-cache FFTs, one thread. Practical experience shows these uncover initial faults fastest.",
                Prime95Mode.Sse, FftPreset.HeavyShort, 1, 6, 2),

            // ─────────────────────────── the long ones ───────────────────────────
            new(isGerman ? "Absicherung – alles über Nacht" : "Overnight - thorough validation",
                isGerman
                    ? "Der vollständige Durchlauf zum Schluss: SSE über alle FFT-Größen, danach AVX, danach AVX2, danach y-cruncher. Die finale Bestätigung, bevor du die Werte ins BIOS überträgst."
                    : "The complete run to finish with: SSE across all FFT sizes, then AVX, then AVX2, then y-cruncher. The final confirmation before you commit the values to the BIOS.",
                Prime95Mode.Sse, FftPreset.All, 1, 10, 3,
                CoreOrder.Alternate,
                [new TestPhase(Prime95Mode.Avx, FftPreset.All),
                 new TestPhase(Prime95Mode.Avx2, FftPreset.All),
                 new TestPhase(Prime95Mode.Avx2, FftPreset.Smallest, EngineKind.YCruncher, YCruncherOptions.CurveOptimizer)]),

            new(isGerman ? "Speicher & Infinity Fabric" : "Memory controller & Infinity Fabric",
                isGerman
                    ? "Große FFTs auf beiden Threads. Belastet Speichercontroller, RAM und FCLK mit — nicht für CO-Feintuning gedacht, sondern um Speicherprobleme auszuschließen."
                    : "Large FFTs on both threads. Also loads the memory controller, RAM and FCLK - not meant for CO fine-tuning, but for ruling out memory problems.",
                Prime95Mode.Avx2, FftPreset.Large, 2, 15, 1, CoreOrder.Sequential),

            // Deliberately the *coolest* load in the set, not the hottest. The failure this
            // reproduces happens at light load: the core boosts to its highest single-core
            // clock, which is exactly where a negative offset has the least voltage to give.
            // AVX2 would heat the core and pull the clock down, away from the failing state.
            new(isGerman ? "⚡ Micro-Bursts – Spiele-Lastwechsel" : "⚡ Micro-bursts - game-style transitions",
                isGerman
                    ? "SSE auf einem Thread, im Takt von Sekundenbruchteilen an- und abgeschaltet: hunderte Lastwechsel pro Minute statt einem Dutzend. Das bildet nach, was eine Spiele-Engine tut — und trifft damit den Fall, den Dauerlast nie erzeugt: der Kern springt aus dem Leerlauf auf Höchsttakt, und die Spannung muss schnell genug mitkommen. Kerne, die stundenlang Prime95 überstehen und dann im Spiel mit WHEA 18 abstürzen, fallen hier durch."
                    : "SSE on one thread, switched on and off in fractions of a second: hundreds of transitions per minute instead of a dozen. It mimics what a game engine does, and so covers the case continuous load never produces — the core jumping from idle to peak boost with the voltage having to keep up. Cores that survive hours of Prime95 and then crash in a game with WHEA 18 fail here.",
                Prime95Mode.Sse, FftPreset.HeavyShort, 1, 10, 2, CoreOrder.Sequential)
            { Transient = TransientMode.MicroBurst },
        };

        // Categories follow the order the list is written in, which is also the order the
        // dropdown shows. Kept as a table rather than repeated on every entry so the shelves
        // stay visible in one place.
        ProfileId[] ids =
        [
            ProfileId.RecommendedCombo, ProfileId.HeavyFfts, ProfileId.FullPrime95, ProfileId.QuickCheck,
            ProfileId.BreakingPoint, ProfileId.HeavyLoad,
            ProfileId.YCruncherCo, ProfileId.YCruncherAll,
            ProfileId.Step1Sse, ProfileId.Step2Avx2, ProfileId.SmallFfts,
            ProfileId.Overnight, ProfileId.MemoryFabric,
            ProfileId.MicroBurst,
        ];

        string[] categories =
        [
            catEveryday, catEveryday, catEveryday, catEveryday,
            catLimit, catLimit,
            catYc, catYc,
            catSteps, catSteps, catSteps,
            catLong, catLong,
            catLimit,
        ];
        // The five that make up the path the program itself recommends, in the order it walks
        // them. Everything else is a variation somebody may want, not a step anybody needs.
        HashSet<ProfileId> essential =
        [
            ProfileId.RecommendedCombo,   // first run: both engines
            ProfileId.HeavyFfts,          // narrowing: one engine, so locked cores are not wasted
            ProfileId.QuickCheck,         // half an hour, for the obvious outliers
            ProfileId.MicroBurst,         // the game-style failure continuous load never produces
            ProfileId.Overnight,          // the proof, before the values go into the BIOS
        ];

        for (int i = 0; i < list.Count && i < categories.Length; i++)
            list[i] = list[i] with
            {
                Category = categories[i],
                Id = ids[i],
                Essential = essential.Contains(ids[i]),
            };

        if (isX3d)
        {
            // Index 2 is the Prime95-only full run.
            list[2] = list[2] with
            {
                Explanation = list[2].Explanation + (isGerman
                    ? " Bei X3D-Prozessoren besonders wichtig: 3D V-Cache reagiert empfindlich auf hohe Temperaturen unter Last."
                    : " Especially important on X3D parts: 3D V-Cache is sensitive to high temperatures under load."),
            };
        }

        return list;
    }
}
