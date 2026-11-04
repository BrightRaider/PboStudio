using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

/// <summary>
/// Every AM4 and AM5 desktop line, because the answer to "what can this processor do" was
/// previously three different lists that disagreed with each other.
/// </summary>
public class CpuModelTests
{
    private static CpuModel Of(string name) => CpuModelService.Detect(name);

    // ── the Ryzen 9000 default ────────────────────────────────────

    /// <summary>
    /// The default for Zen 4 and newer (Ryzen 7000, 8000 and 9000) is -50. The vendored ZenStates
    /// library documents the Curve Optimizer range as "-30 to 30 before Zen 4, and from -50 to 50 on
    /// Zen 4 and newer" in the commands this program sends. 1.1.2 applied that to Zen 5 only and left
    /// Zen 4 at -30, which cuts a part that takes -50 off at -30: the search ends at a depth the
    /// board would have accepted. Before that the figure was removed in 1.1.0 on an unsourced claim
    /// and came back in 1.1.1. What makes it safe is not the number but what surrounds it: every
    /// write beyond -30 is read back, a value the processor does not keep ends the search at the
    /// last value that held, and that depth is remembered. See CurveOptimizerRange and the
    /// runner's depth-limit handling.
    /// </summary>
    [Theory]
    [InlineData("AMD Ryzen 9 9950X3D 16-Core Processor", CpuFamily.Zen5)]
    [InlineData("AMD Ryzen 9 9950X 16-Core Processor", CpuFamily.Zen5)]
    [InlineData("AMD Ryzen 9 9900X 12-Core Processor", CpuFamily.Zen5)]
    [InlineData("AMD Ryzen 7 9800X3D 8-Core Processor", CpuFamily.Zen5)]
    [InlineData("AMD Ryzen 7 9700X 8-Core Processor", CpuFamily.Zen5)]
    [InlineData("AMD Ryzen 5 9600X 6-Core Processor", CpuFamily.Zen5)]
    [InlineData("AMD Ryzen 9 7950X3D 16-Core Processor", CpuFamily.Zen4)]
    [InlineData("AMD Ryzen 9 7900X3D 12-Core Processor", CpuFamily.Zen4)]
    [InlineData("AMD Ryzen 7 7800X3D 8-Core Processor", CpuFamily.Zen4)]
    [InlineData("AMD Ryzen 7 7700X 8-Core Processor", CpuFamily.Zen4)]
    [InlineData("AMD Ryzen 5 7600X 6-Core Processor", CpuFamily.Zen4)]
    public void Zen4AndNewerPartsDefaultToMinusFifty(string name, CpuFamily family)
    {
        var cpu = Of(name);

        Assert.Equal(family, cpu.Family);
        Assert.Equal(-50, cpu.MaxNegativeMargin);
        Assert.Equal(-50, AutoTunerService.GetMaxNegativeMargin(name));
    }

    /// <summary>The Curve Shaper is a Zen 5 feature; the deeper range is not tied to it.</summary>
    [Theory]
    [InlineData("AMD Ryzen 9 9950X 16-Core Processor", true)]
    [InlineData("AMD Ryzen 9 7950X3D 16-Core Processor", false)]
    public void TheCurveShaperBelongsToZen5Only(string name, bool expected)
    {
        Assert.Equal(expected, Of(name).HasCurveShaper);
    }

    /// <summary>Before Zen 4 the range is -30 to 30, per the same source, so the Zen 3 parts stay at the standard.</summary>
    [Theory]
    [InlineData("AMD Ryzen 9 5950X 16-Core Processor")]
    [InlineData("AMD Ryzen 7 5800X3D 8-Core Processor")]
    [InlineData("AMD Ryzen 5 5600X 6-Core Processor")]
    public void Zen3PartsStayAtTheStandard(string name)
    {
        Assert.Equal(-30, Of(name).MaxNegativeMargin);
        Assert.Equal(-30, AutoTunerService.GetMaxNegativeMargin(name));
    }

    /// <summary>A processor without the feature has no depth to speak of, whatever generation it is.</summary>
    [Fact]
    public void APartWithoutTheFeatureHasNoDepth()
    {
        Assert.Equal(0, Of("AMD Ryzen 7 2700X Eight-Core Processor").MaxNegativeMargin);
    }

    // ── AM4 ───────────────────────────────────────────────────────

    /// <summary>
    /// Curve Optimizer arrived with Zen 3. On the older AM4 parts PBO offers its scalar and its
    /// power limits and nothing per core, so running a weekend-long search on one would measure
    /// a setting that does not exist.
    /// </summary>
    [Theory]
    [InlineData("AMD Ryzen 7 1800X Eight-Core Processor", CpuFamily.Zen1)]
    [InlineData("AMD Ryzen 5 1600 Six-Core Processor", CpuFamily.Zen1)]
    [InlineData("AMD Ryzen 7 2700X Eight-Core Processor", CpuFamily.ZenPlus)]
    [InlineData("AMD Ryzen 5 2600 Six-Core Processor", CpuFamily.ZenPlus)]
    [InlineData("AMD Ryzen 9 3950X 16-Core Processor", CpuFamily.Zen2)]
    [InlineData("AMD Ryzen 7 3700X 8-Core Processor", CpuFamily.Zen2)]
    [InlineData("AMD Ryzen 5 3600 6-Core Processor", CpuFamily.Zen2)]
    public void TheOlderAm4PartsHaveNoCurveOptimizer(string name, CpuFamily family)
    {
        var cpu = Of(name);

        Assert.Equal(family, cpu.Family);
        Assert.Equal(CpuSocket.AM4, cpu.Socket);
        Assert.False(cpu.SupportsCurveOptimizer);
    }

    [Theory]
    [InlineData("AMD Ryzen 9 5950X 16-Core Processor")]
    [InlineData("AMD Ryzen 9 5900X 12-Core Processor")]
    [InlineData("AMD Ryzen 7 5800X 8-Core Processor")]
    [InlineData("AMD Ryzen 7 5700X 8-Core Processor")]
    [InlineData("AMD Ryzen 5 5600X 6-Core Processor")]
    [InlineData("AMD Ryzen 5 5500 6-Core Processor")]
    public void Zen3OnAm4HasTheFullRange(string name)
    {
        var cpu = Of(name);

        Assert.Equal(CpuFamily.Zen3, cpu.Family);
        Assert.Equal(CpuSocket.AM4, cpu.Socket);
        Assert.True(cpu.SupportsCurveOptimizer);
        Assert.Equal(-30, cpu.MaxNegativeMargin);
        Assert.False(cpu.NegativeOnly);
    }

    /// <summary>
    /// The Zen 3 X3D parts take an undervolt and nothing else — no positive offset, no
    /// multiplier, no voltage control. Later X3D designs put the cache under the cores and
    /// dropped the restriction, so this is not an X3D property, it is a Zen 3 X3D property.
    /// </summary>
    [Theory]
    [InlineData("AMD Ryzen 7 5800X3D 8-Core Processor")]
    [InlineData("AMD Ryzen 7 5700X3D 8-Core Processor")]
    [InlineData("AMD Ryzen 5 5600X3D 6-Core Processor")]
    public void TheZen3X3dPartsTakeNegativeValuesOnly(string name)
    {
        var cpu = Of(name);

        Assert.Equal(CpuFamily.Zen3, cpu.Family);
        Assert.True(cpu.IsX3d);
        Assert.True(cpu.NegativeOnly);
        Assert.Equal(-30, cpu.MaxNegativeMargin);
    }

    [Theory]
    [InlineData("AMD Ryzen 7 9800X3D 8-Core Processor")]
    [InlineData("AMD Ryzen 7 7800X3D 8-Core Processor")]
    public void TheLaterX3dPartsAreNotRestrictedToNegativeValues(string name)
    {
        var cpu = Of(name);

        Assert.True(cpu.IsX3d);
        Assert.False(cpu.NegativeOnly);
    }

    /// <summary>
    /// The APUs break the "leading digit is the generation" rule in both directions: a 3400G is
    /// Zen+ while a 3700X is Zen 2, and a 5700G is Zen 3 sharing its leading digit with Vermeer.
    /// </summary>
    [Theory]
    [InlineData("AMD Ryzen 5 2400G with Radeon Vega Graphics", CpuFamily.Zen1, false)]
    [InlineData("AMD Ryzen 5 3400G with Radeon Vega Graphics", CpuFamily.ZenPlus, false)]
    [InlineData("AMD Ryzen 5 4650G with Radeon Graphics", CpuFamily.Zen2, false)]
    [InlineData("AMD Ryzen 7 5700G with Radeon Graphics", CpuFamily.Zen3, true)]
    [InlineData("AMD Ryzen 5 5600G with Radeon Graphics", CpuFamily.Zen3, true)]
    public void TheApusAreSortedByTheirOwnTable(string name, CpuFamily family, bool hasCo)
    {
        var cpu = Of(name);

        Assert.Equal(family, cpu.Family);
        Assert.True(cpu.IsApu);
        Assert.Equal(hasCo, cpu.SupportsCurveOptimizer);
    }

    // ── AM5 ───────────────────────────────────────────────────────

    [Theory]
    [InlineData("AMD Ryzen 9 7950X3D 16-Core Processor")]
    [InlineData("AMD Ryzen 9 7900X 12-Core Processor")]
    [InlineData("AMD Ryzen 7 7800X3D 8-Core Processor")]
    [InlineData("AMD Ryzen 7 7700X 8-Core Processor")]
    [InlineData("AMD Ryzen 5 7600X 6-Core Processor")]
    [InlineData("AMD Ryzen 5 7500F 6-Core Processor")]
    public void Zen4OnAm5HasTheFullRange(string name)
    {
        var cpu = Of(name);

        Assert.Equal(CpuFamily.Zen4, cpu.Family);
        Assert.Equal(CpuSocket.AM5, cpu.Socket);
        Assert.Equal(-50, cpu.MaxNegativeMargin);   // 1.1.3: Zen 4 and newer, see Zen4AndNewerPartsDefaultToMinusFifty
        Assert.False(cpu.HasCurveShaper);
    }

    [Theory]
    [InlineData("AMD Ryzen 7 8700G w/ Radeon 780M Graphics")]
    [InlineData("AMD Ryzen 5 8600G w/ Radeon 760M Graphics")]
    public void TheAm5ApusAreZen4AndTunable(string name)
    {
        var cpu = Of(name);

        Assert.Equal(CpuFamily.Zen4, cpu.Family);
        Assert.Equal(CpuSocket.AM5, cpu.Socket);
        Assert.True(cpu.SupportsCurveOptimizer);
    }

    // ── the things that read this ─────────────────────────────────

    /// <summary>
    /// y-cruncher ships a separate binary per architecture, and picking the wrong one measures
    /// the wrong instruction mix. Zen 1 and Zen+ share one, which is why they collapse here.
    /// </summary>
    [Theory]
    [InlineData("AMD Ryzen 7 9800X3D 8-Core Processor", ZenGeneration.Zen5)]
    [InlineData("AMD Ryzen 7 7800X3D 8-Core Processor", ZenGeneration.Zen4)]
    [InlineData("AMD Ryzen 7 5800X3D 8-Core Processor", ZenGeneration.Zen3)]
    [InlineData("AMD Ryzen 7 3700X 8-Core Processor", ZenGeneration.Zen2)]
    [InlineData("AMD Ryzen 7 2700X Eight-Core Processor", ZenGeneration.Zen1)]
    public void TheYCruncherBinaryFollowsTheFamily(string name, ZenGeneration expected)
    {
        Assert.Equal(expected, Of(name).YCruncherGeneration);
    }

    /// <summary>
    /// A processor released after this list was written still has to work. Assuming the standard
    /// range is a smaller failure than refusing to run.
    /// </summary>
    [Fact]
    public void AnUnknownNameGetsTheStandardRangeRatherThanNothing()
    {
        var cpu = Of("AMD Ryzen 7 11800X 8-Core Processor");

        Assert.True(cpu.SupportsCurveOptimizer);
        Assert.Equal(-30, cpu.MaxNegativeMargin);
    }

    [Fact]
    public void AnEmptyNameDoesNotThrow()
    {
        var cpu = Of("");

        Assert.Equal(CpuFamily.Unknown, cpu.Family);
        Assert.Equal(-30, AutoTunerService.GetMaxNegativeMargin(""));
    }

    /// <summary>The starting point is short of the limit on purpose, and shorter on an X3D.</summary>
    [Fact]
    public void TheSuggestedStartStaysWellAboveTheChipLimit()
    {
        foreach (var name in new[]
        {
            "AMD Ryzen 7 9700X 8-Core Processor",
            "AMD Ryzen 7 7700X 8-Core Processor",
            "AMD Ryzen 7 5800X 8-Core Processor",
        })
        {
            var cpu = Of(name);
            Assert.True(cpu.SuggestedStart > cpu.MaxNegativeMargin);
        }

        Assert.True(Of("AMD Ryzen 7 5800X3D 8-Core Processor").SuggestedStart
                  > Of("AMD Ryzen 7 5800X 8-Core Processor").SuggestedStart);
    }

    [Fact]
    public void ThePartsWithoutTheFeatureSaySoInWords()
    {
        string text = CpuModelService.Describe(Of("AMD Ryzen 7 2700X Eight-Core Processor"), isGerman: false);

        Assert.Contains("no Curve Optimizer", text);
        Assert.Contains("Zen 3", text);
    }
}
