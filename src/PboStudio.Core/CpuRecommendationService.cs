namespace PboStudio.Core;

public sealed record CpuTuningRecommendation(
    string CpuName,
    ZenGeneration Generation,
    bool IsX3D,
    string RecommendedProfileName,
    int DefaultStartingCo,
    int X3dStartingCo,
    string SummaryAdvice,
    IReadOnlyDictionary<int, int> SuggestedCoreMargins,
    IReadOnlyList<string> WheaCorrections
);

/// <summary>
/// Smart CPU tuning & WHEA error recommendation engine.
/// Provides tailored Curve Optimizer starting values and dynamically adjusts core margins
/// relative to currently configured values when WHEA hardware errors occur.
/// Supports both Grob (+4) and Fein (+2) mitigation precision modes.
/// </summary>
public static class CpuRecommendationService
{
    public static CpuTuningRecommendation GetRecommendation(
        string cpuName,
        IReadOnlyList<PhysicalCore> cores,
        IReadOnlyList<WheaEvent>? wheaEvents = null,
        IReadOnlyDictionary<int, int>? currentMargins = null,
        bool isFineMode = false,
        bool? isGermanOverride = null)
    {
        // Was `bool isGerman = false`, and the one production caller never passed it - so the
        // whole recommendation stayed English no matter what the user picked.
        bool isGerman = isGermanOverride ?? LocalizationService.IsGerman;

        bool isX3D = cpuName.Contains("X3D", StringComparison.OrdinalIgnoreCase);

        ZenGeneration gen = ZenGeneration.Unknown;
        if (cpuName.Contains("9950") || cpuName.Contains("9900") || cpuName.Contains("9800") || cpuName.Contains("9700") || cpuName.Contains("9600"))
            gen = ZenGeneration.Zen5;
        else if (cpuName.Contains("7950") || cpuName.Contains("7900") || cpuName.Contains("7800") || cpuName.Contains("7700") || cpuName.Contains("7600") || cpuName.Contains("7500"))
            gen = ZenGeneration.Zen4;
        else if (cpuName.Contains("5950") || cpuName.Contains("5900") || cpuName.Contains("5800") || cpuName.Contains("5700") || cpuName.Contains("5600") || cpuName.Contains("5500"))
            gen = ZenGeneration.Zen3;
        else if (cpuName.Contains("3950") || cpuName.Contains("3900") || cpuName.Contains("3800") || cpuName.Contains("3700") || cpuName.Contains("3600") || cpuName.Contains("3500"))
            gen = ZenGeneration.Zen2;
        else
            gen = ZenGeneration.Zen4;

        int baseCo = gen switch
        {
            ZenGeneration.Zen5 => isX3D ? -15 : -20,
            ZenGeneration.Zen4 => isX3D ? -15 : -22,
            ZenGeneration.Zen3 => isX3D ? -10 : -18,
            ZenGeneration.Zen2 => -10,
            _ => -15
        };

        string profileName = gen switch
        {
            ZenGeneration.Zen5 => isGerman ? "Komplett – SSE und AVX2 am Stück (empfohlen)" : "Full Run - SSE & AVX2 Combined (Recommended)",
            ZenGeneration.Zen4 => isGerman ? "Komplett – SSE und AVX2 am Stück (empfohlen)" : "Full Run - SSE & AVX2 Combined (Recommended)",
            ZenGeneration.Zen3 => isX3D ? (isGerman ? "Schritt 2 – AVX2 (danach)" : "Step 2 - AVX2 Heavy Load") : (isGerman ? "Komplett – SSE und AVX2 am Stück (empfohlen)" : "Full Run - SSE & AVX2 Combined (Recommended)"),
            _ => isGerman ? "Schritt 1 – SSE (fängt hier an)" : "Step 1 - SSE (Start here)"
        };

        string modeLabel = isFineMode
            ? (isGerman ? "Fein-Modus (+2)" : "Fine Mode (+2)")
            : (isGerman ? "Grob-Modus (+4)" : "Coarse Mode (+4)");

        string summary = isGerman
            ? gen switch
            {
                ZenGeneration.Zen5 => $"Für deinen {cpuName} (Zen 5): Starte mit ca. {baseCo} CO auf allen Kernen. Modus: {modeLabel}.",
                ZenGeneration.Zen4 => $"Für deinen {cpuName} (Zen 4): Empfohlenes Start-CO ist {baseCo}. {(isX3D ? "Bei X3D-Kernen vorsichtiger vorgehen (-15 max)." : "Standard-Kerne vertragen oft -20 bis -28.")}",
                ZenGeneration.Zen3 => $"Für deinen {cpuName} (Zen 3): Empfohlener Richtwert ist {baseCo}. WHEA-Korrektur: {modeLabel}.",
                _ => $"Für deinen {cpuName}: Konservativer Startwert bei {baseCo} empfohlen."
            }
            : gen switch
            {
                ZenGeneration.Zen5 => $"For your {cpuName} (Zen 5): Start with ~{baseCo} CO on all cores. Mode: {modeLabel}.",
                ZenGeneration.Zen4 => $"For your {cpuName} (Zen 4): Recommended starting CO is {baseCo}. {(isX3D ? "Exercise caution with X3D cores (-15 max)." : "Standard cores often handle -20 to -28.")}",
                ZenGeneration.Zen3 => $"For your {cpuName} (Zen 3): Recommended target value is {baseCo}. WHEA mitigation: {modeLabel}.",
                _ => $"For your {cpuName}: Conservative starting offset of {baseCo} recommended."
            };

        var suggestedMargins = new Dictionary<int, int>();
        foreach (var core in cores)
        {
            int startingVal = currentMargins != null && currentMargins.TryGetValue(core.Index, out int cur)
                ? cur
                : baseCo;
            suggestedMargins[core.Index] = startingVal;
        }

        var corrections = new List<string>();
        int backoffStep = isFineMode ? 2 : 4;

        // Correlate WHEA Events to physical cores
        if (wheaEvents != null && wheaEvents.Count > 0)
        {
            var wheaPerCore = new Dictionary<int, int>();
            foreach (var evt in wheaEvents)
            {
                if (evt.ApicId.HasValue)
                {
                    int coreIdx = WheaWatcher.CoreForApicId(evt.ApicId.Value, cores) ?? (evt.ApicId.Value / 2);
                    if (coreIdx >= 0 && coreIdx < cores.Count)
                    {
                        wheaPerCore[coreIdx] = wheaPerCore.GetValueOrDefault(coreIdx, 0) + 1;
                    }
                }
            }

            foreach (var (coreIdx, count) in wheaPerCore.OrderBy(k => k.Key))
            {
                int currentVal = currentMargins != null && currentMargins.TryGetValue(coreIdx, out int uVal)
                    ? uVal
                    : baseCo;

                int correctedVal = Math.Min(0, currentVal + (count * backoffStep));
                suggestedMargins[coreIdx] = correctedVal;

                if (isGerman)
                    corrections.Add($"⚠️ Kern {coreIdx}: {count}x WHEA-Fehler! Aktuell: {currentVal} ➔ Empfohlen: {correctedVal} (+{count * backoffStep} {(isFineMode ? "Fein" : "Grob")}).");
                else
                    corrections.Add($"⚠️ Core {coreIdx}: {count}x WHEA Error! Current: {currentVal} ➔ Recommended: {correctedVal} (+{count * backoffStep} {(isFineMode ? "Fine" : "Coarse")}).");
            }
        }

        return new CpuTuningRecommendation(
            cpuName,
            gen,
            isX3D,
            profileName,
            baseCo,
            isX3D ? -12 : baseCo,
            summary,
            suggestedMargins,
            corrections
        );
    }
}
