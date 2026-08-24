namespace PboStudio.Core;

/// <summary>One leg of a run: an engine, instruction set, and FFT range or algorithm preset.</summary>
public sealed record TestPhase(
    Prime95Mode Mode = Prime95Mode.Sse,
    FftPreset Fft = FftPreset.Huge,
    EngineKind Engine = EngineKind.Prime95,
    YCruncherOptions? YcOptions = null)
{
    public string Label => Engine == EngineKind.YCruncher
        ? "y-cruncher (CO-Preset)"
        : $"{Mode.ToString().ToUpperInvariant()} / {Fft}";
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
    EngineKind Engine = EngineKind.Prime95)
{
    /// <summary>
    /// Every leg this profile runs, in order. Most profiles are a single leg; the combined ones
    /// chain several so nobody has to reconfigure and restart between instruction sets or engines.
    /// </summary>
    public IReadOnlyList<TestPhase> Phases =>
        [new TestPhase(Mode, Fft, Engine), .. ExtraPhases ?? []];

    public override string ToString() => Name;
}

/// <summary>
/// Ready-made settings, so nobody has to guess which knobs matter.
/// </summary>
public static class TestProfiles
{
    public static IReadOnlyList<TestProfile> For(string cpuName, bool isGerman = false)
    {
        bool isX3d = cpuName.Contains("X3D", StringComparison.OrdinalIgnoreCase);

        var list = new List<TestProfile>
        {
            new(isGerman ? "🏆 Hybrid Ultimate – Prime95 SSE & y-cruncher (Empfohlen)" : "🏆 Hybrid Ultimate - Prime95 SSE & y-cruncher (Recommended)",
                isGerman
                    ? "Der Gold-Standard aus CoreCycler: Kombiniert Prime95 SSE für maximale Taktfrequenz & Vdroop-Limits mit y-cruncher AVX für schwere Vektor- & Cache-Stabilität."
                    : "The gold standard from CoreCycler: Combines Prime95 SSE for peak boost & transient Vdroop limits with y-cruncher AVX for heavy vector and cache validation.",
                Prime95Mode.Sse, FftPreset.Huge, 1, 6, 3,
                CoreOrder.Alternate,
                [new TestPhase(Prime95Mode.Avx2, FftPreset.Smallest, EngineKind.YCruncher, YCruncherOptions.CurveOptimizer)]),

            new(isGerman ? "Komplett – Prime95 SSE & AVX2" : "Full Run - SSE & AVX2 Combined",
                isGerman
                    ? "Läuft beide nötigen Modi nacheinander durch: erst SSE mit großen FFTs für Grenzfälle bei Höchsttakt, dann AVX2 mit kleinen FFTs. Prüft den Kern vollständig."
                    : "Runs both essential stress modes sequentially: first SSE with large FFTs for max boost limits, then AVX2 with small FFTs. Completely validates core stability.",
                Prime95Mode.Sse, FftPreset.Huge, 1, 6, 3,
                CoreOrder.Alternate,
                [new TestPhase(Prime95Mode.Avx2, FftPreset.Smallest)]),

            new(isGerman ? "Absicherung – Alles über Nacht" : "Overnight Thorough Validation",
                isGerman
                    ? "Der vollständige Durchlauf zum Schluss: SSE über alle FFT-Größen, danach AVX, danach AVX2. Die finale Bestätigung für deine Curve Optimizer Werte."
                    : "Comprehensive overnight test: SSE across all FFT sizes, followed by AVX and AVX2. The ultimate verification for your Curve Optimizer settings.",
                Prime95Mode.Sse, FftPreset.All, 1, 10, 3,
                CoreOrder.Alternate,
                [new TestPhase(Prime95Mode.Avx, FftPreset.All),
                 new TestPhase(Prime95Mode.Avx2, FftPreset.All)]),

            new(isGerman ? "Schritt 1 – SSE (Fängt hier an)" : "Step 1 - SSE (Start Here)",
                isGerman
                    ? "SSE mit riesigen FFTs, ein Thread, 6 Minuten pro Kern. Lässt den Kern bei geringer Spannung maximal hoch boosten."
                    : "SSE with huge FFTs, 1 thread, 6 mins per core. Allows cores to boost to maximum clock rates at lowest voltage points.",
                Prime95Mode.Sse, FftPreset.Huge, 1, 6, 3),

            new(isGerman ? "Schritt 2 – AVX2 (Danach)" : "Step 2 - AVX2 Heavy Load",
                isGerman
                    ? "Dieselbe Runde mit AVX2. Beansprucht Schaltungen, die SSE nie anfasst, und zieht deutlich mehr Strom."
                    : "Identical run with AVX2. Stresses execution units that SSE never touches, drawing higher power and heat.",
                Prime95Mode.Avx2, FftPreset.Smallest, 1, 6, 3),

            new(isGerman ? "Schnellcheck (3 Min / Kern)" : "Quick Check (3 Mins / Core)",
                isGerman
                    ? "3 Minuten pro Kern, ein Durchgang, SSE. Findet nur die groben Ausreißer in ca. 30 Minuten."
                    : "3 minutes per core, single pass, SSE. Quickly flags major voltage instabilities in under 30 minutes.",
                Prime95Mode.Sse, FftPreset.Huge, 1, 3, 1),

            new(isGerman ? "Kleine FFTs (Schnellsuche)" : "Small FFTs (Fast Discovery)",
                isGerman
                    ? "Cache-interne FFTs. Deckt anfängliche Fehler in der Praxis am schnellsten auf."
                    : "In-cache FFTs. Practical experience shows these uncover initial voltage drops fastest.",
                Prime95Mode.Sse, FftPreset.Smallest, 1, 6, 2),

            new(isGerman ? "Volllast – Kühlung & Limits" : "Heavy Load - Thermal & Power Limits",
                isGerman
                    ? "AVX2 auf beiden Threads. Testet ob Kühlung und PPT/TDC/EDC PBO-Grenzen ausreichen."
                    : "AVX2 across all SMT threads. Tests thermal headroom and PBO power limits (PPT/TDC/EDC).",
                Prime95Mode.Avx2, FftPreset.Smallest, 2, 10, 1),

            new(isGerman ? "Speicher & Infinity Fabric" : "Memory Controller & Infinity Fabric",
                isGerman
                    ? "Große FFTs auf beiden Threads für Speichercontroller, RAM- und FCLK-Stabilität."
                    : "Large FFTs across all threads to validate RAM speed, memory controller, and FCLK stability.",
                Prime95Mode.Avx2, FftPreset.Large, 2, 15, 1, CoreOrder.Sequential),
        };

        if (isX3d)
        {
            list[1] = list[1] with
            {
                Explanation = list[1].Explanation + (isGerman
                    ? " Bei X3D-Prozessoren besonders wichtig: 3D V-Cache Prozessoren reagieren empfindlich auf hohe Temperaturen unter Last."
                    : " Critical for X3D CPUs: 3D V-Cache processors are temperature-sensitive under heavy multi-threaded AVX2 loads."),
            };
        }

        return list;
    }
}
