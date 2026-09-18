using System.Text.RegularExpressions;

namespace PboStudio.Core;

public enum CpuFamily { Unknown, Zen1, ZenPlus, Zen2, Zen3, Zen4, Zen5 }

public enum CpuSocket { Unknown, AM4, AM5, ThreadRipper }

/// <summary>
/// What this particular processor can actually do, worked out from its name.
///
/// The program had three separate answers to "which Zen is this": a generation detector in the
/// window for picking a y-cruncher binary, a limit table in the auto-tuner, and a third list in
/// the recommendation service that also named profiles by display string. They disagreed. The
/// auto-tuner's table was the expensive one — it let the search walk a Ryzen 9000 down to -50,
/// a number the Curve Optimizer cannot express on any AMD desktop part, which makes the values
/// it ends up reporting impossible to type into a BIOS.
/// </summary>
/// <param name="SupportsCurveOptimizer">
/// False on Zen 1, Zen+ and Zen 2. Curve Optimizer arrived with Zen 3; on the older parts PBO
/// has only its scalar and power limits, so there is nothing here to tune.
/// </param>
/// <param name="MaxNegativeMargin">
/// The most negative per-core offset the Curve Optimizer accepts. It is -30 on every desktop
/// part that has the feature at all. Zen 5 adds Curve Shaper, which is a second, separate set
/// of offsets across temperature and frequency bands — not a wider range for this one.
/// </param>
/// <param name="NegativeOnly">
/// True for the Zen 3 X3D parts, which accept undervolting but no positive offset and no
/// multiplier or voltage overclocking at all.
/// </param>
public sealed record CpuModel(
    string Name,
    CpuFamily Family,
    CpuSocket Socket,
    bool IsX3d,
    bool IsApu,
    bool SupportsCurveOptimizer,
    int MaxNegativeMargin,
    bool NegativeOnly = false,
    bool HasCurveShaper = false)
{
    /// <summary>
    /// How far a first attempt should reach before the search refines it. Deliberately short of
    /// the chip limit: the point is a value most cores clear, not the best one.
    /// </summary>
    public int SuggestedStart => !SupportsCurveOptimizer ? 0 : IsX3d ? -10 : Family switch
    {
        CpuFamily.Zen5 => -15,
        CpuFamily.Zen4 => -15,
        CpuFamily.Zen3 => -15,
        _ => -10,
    };

    /// <summary>
    /// The generation y-cruncher's binary picker wants. Zen 1 and Zen+ share a binary, which is
    /// why the two collapse here and nowhere else.
    /// </summary>
    public ZenGeneration YCruncherGeneration => Family switch
    {
        CpuFamily.Zen5 => ZenGeneration.Zen5,
        CpuFamily.Zen4 => ZenGeneration.Zen4,
        CpuFamily.Zen3 => ZenGeneration.Zen3,
        CpuFamily.Zen2 => ZenGeneration.Zen2,
        CpuFamily.Zen1 or CpuFamily.ZenPlus => ZenGeneration.Zen1,
        _ => ZenGeneration.Zen4,
    };
}

public static class CpuModelService
{
    public const int CurveOptimizerLimit = -30;

    /// <summary>
    /// Exactly four digits, which is how every Ryzen model number is written. The negative
    /// lookahead matters: without it "11800X" matches as "1180", and a part released after this
    /// list was written gets filed as a first-generation Ryzen with no Curve Optimizer.
    /// </summary>
    private static readonly Regex ModelNumber =
        new(@"\b(\d{4})(?!\d)\s*([A-Za-z0-9]*)", RegexOptions.Compiled);

    /// <summary>
    /// The APUs, which break the "leading digit is the generation" rule in both directions: a
    /// 3400G is Zen+ while a 3700X is Zen 2, and a 5700G is Zen 3 while sharing its leading
    /// digit with Vermeer, which it is not.
    /// </summary>
    private static readonly Dictionary<int, CpuFamily> ApuFamilies = new()
    {
        [2200] = CpuFamily.Zen1,  [2400] = CpuFamily.Zen1,
        [3200] = CpuFamily.ZenPlus, [3400] = CpuFamily.ZenPlus,
        [4300] = CpuFamily.Zen2,  [4350] = CpuFamily.Zen2,  [4600] = CpuFamily.Zen2,
        [4650] = CpuFamily.Zen2,  [4700] = CpuFamily.Zen2,  [4750] = CpuFamily.Zen2,
        [5300] = CpuFamily.Zen3,  [5350] = CpuFamily.Zen3,  [5600] = CpuFamily.Zen3,
        [5650] = CpuFamily.Zen3,  [5700] = CpuFamily.Zen3,  [5750] = CpuFamily.Zen3,
        [8500] = CpuFamily.Zen4,  [8600] = CpuFamily.Zen4,  [8700] = CpuFamily.Zen4,
    };

    public static CpuModel Detect(string cpuName)
    {
        string name = cpuName ?? "";
        bool isX3d = name.Contains("X3D", StringComparison.OrdinalIgnoreCase);

        var match = ModelNumber.Match(name);
        if (!match.Success)
            return Unknown(name, isX3d);

        int number = int.Parse(match.Groups[1].Value);
        string suffix = match.Groups[2].Value.ToUpperInvariant();

        bool isApu = suffix.StartsWith('G') || suffix.StartsWith("GE");
        bool isThreadripper = name.Contains("Threadripper", StringComparison.OrdinalIgnoreCase)
            || suffix.StartsWith("WX");

        var family = FamilyOf(number, isApu);
        if (family == CpuFamily.Unknown) return Unknown(name, isX3d);

        // Curve Optimizer arrived with Zen 3. On anything older PBO offers its scalar and its
        // power limits and nothing per-core, so there is no curve to optimise.
        bool hasCo = family is CpuFamily.Zen3 or CpuFamily.Zen4 or CpuFamily.Zen5;

        var socket = isThreadripper
            ? CpuSocket.ThreadRipper
            : family is CpuFamily.Zen4 or CpuFamily.Zen5 ? CpuSocket.AM5 : CpuSocket.AM4;

        return new CpuModel(
            name,
            family,
            socket,
            isX3d,
            isApu,
            hasCo,
            hasCo ? CurveOptimizerLimit : 0,

            // The Zen 3 X3D parts take an undervolt and nothing else: no positive offset, no
            // multiplier, no voltage control. Later X3D designs put the cache under the cores
            // and dropped the restriction.
            NegativeOnly: hasCo && isX3d && family == CpuFamily.Zen3,
            HasCurveShaper: family == CpuFamily.Zen5);
    }

    /// <summary>
    /// Model number to family. The leading digit names the generation for the desktop parts;
    /// the APUs are a lookup because they do not follow it.
    /// </summary>
    private static CpuFamily FamilyOf(int number, bool isApu)
    {
        if (isApu && ApuFamilies.TryGetValue(number, out var apu)) return apu;

        return number switch
        {
            >= 1000 and < 2000 => CpuFamily.Zen1,
            >= 2000 and < 3000 => CpuFamily.ZenPlus,
            >= 3000 and < 4000 => CpuFamily.Zen2,
            >= 4000 and < 5000 => CpuFamily.Zen2,     // Renoir APUs, when the suffix was missed
            >= 5000 and < 6000 => CpuFamily.Zen3,
            >= 7000 and < 8000 => CpuFamily.Zen4,
            >= 8000 and < 9000 => CpuFamily.Zen4,     // Phoenix APUs on AM5
            >= 9000 and < 10000 => CpuFamily.Zen5,
            _ => CpuFamily.Unknown,                   // 6000 is mobile-only; never an AM4/AM5 part
        };
    }

    /// <summary>
    /// An unrecognised name is assumed to have a Curve Optimizer with the standard range. The
    /// alternative is refusing to work on a processor released after this list was written,
    /// which is a worse failure than being cautious about its limit.
    /// </summary>
    private static CpuModel Unknown(string name, bool isX3d) =>
        new(name, CpuFamily.Unknown, CpuSocket.Unknown, isX3d, IsApu: false,
            SupportsCurveOptimizer: true, CurveOptimizerLimit);

    /// <summary>One line saying what this processor is and what that means for tuning it.</summary>
    public static string Describe(CpuModel cpu, bool isGerman)
    {
        if (!cpu.SupportsCurveOptimizer)
            return isGerman
                ? $"{FamilyName(cpu.Family)} hat keinen Curve Optimizer — der kam erst mit Zen 3. PBO lässt sich hier nur über Scalar und die Leistungsgrenzen beeinflussen, nicht pro Kern."
                : $"{FamilyName(cpu.Family)} has no Curve Optimizer — it arrived with Zen 3. PBO here is only the scalar and the power limits, nothing per core.";

        var parts = new List<string>
        {
            isGerman
                ? $"{FamilyName(cpu.Family)}, Curve Optimizer bis {cpu.MaxNegativeMargin}"
                : $"{FamilyName(cpu.Family)}, Curve Optimizer down to {cpu.MaxNegativeMargin}",
        };

        if (cpu.IsX3d)
            parts.Add(isGerman
                ? "X3D: der gestapelte Cache reagiert empfindlich auf Temperatur, deshalb steht SSE vor AVX2"
                : "X3D: the stacked cache is sensitive to temperature, so SSE leads before AVX2");

        if (cpu.NegativeOnly)
            parts.Add(isGerman
                ? "nimmt nur negative Werte an"
                : "accepts negative values only");

        if (cpu.HasCurveShaper)
            parts.Add(isGerman
                ? "Curve Shaper ist eine zweite, getrennte Kurve im BIOS — PboStudio setzt den Curve Optimizer"
                : "Curve Shaper is a second, separate curve in the BIOS — PboStudio sets the Curve Optimizer");

        return string.Join(isGerman ? ". " : ". ", parts) + ".";
    }

    public static string FamilyName(CpuFamily family) => family switch
    {
        CpuFamily.Zen1 => "Zen 1",
        CpuFamily.ZenPlus => "Zen+",
        CpuFamily.Zen2 => "Zen 2",
        CpuFamily.Zen3 => "Zen 3",
        CpuFamily.Zen4 => "Zen 4",
        CpuFamily.Zen5 => "Zen 5",
        _ => "Ryzen",
    };
}
