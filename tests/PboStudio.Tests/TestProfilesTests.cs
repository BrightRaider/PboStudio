using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

public class TestProfilesTests
{
    [Fact]
    public void For_NonX3d_ReturnsStandardProfiles()
    {
        var profiles = TestProfiles.For("AMD Ryzen 7 7700X", isGerman: false);

        Assert.NotEmpty(profiles);
        Assert.Contains(profiles, p => p.Name.Contains("Hybrid Ultimate"));
        Assert.Contains(profiles, p => p.Name.Contains("Full Run"));
        Assert.Contains(profiles, p => p.Name.Contains("Step 1 - SSE"));
        Assert.Contains(profiles, p => p.Name.Contains("Step 2 - AVX2"));
    }

    [Fact]
    public void For_X3d_IncludesThermalAdviceInExplanation()
    {
        var profilesDe = TestProfiles.For("AMD Ryzen 7 7800X3D", isGerman: true);
        Assert.Contains("3D V-Cache", profilesDe[1].Explanation);

        var profilesEn = TestProfiles.For("AMD Ryzen 7 7800X3D", isGerman: false);
        Assert.Contains("3D V-Cache", profilesEn[1].Explanation);
    }

    [Fact]
    public void TestProfile_Phases_ChainsBaseAndExtraPhases()
    {
        var profile = new TestProfile(
            "Multi Phase", "Explanation",
            Prime95Mode.Sse, FftPreset.Huge, 1, 6, 3,
            CoreOrder.Alternate,
            [new TestPhase(Prime95Mode.Avx2, FftPreset.Smallest)]);

        Assert.Equal(2, profile.Phases.Count);
        Assert.Equal(Prime95Mode.Sse, profile.Phases[0].Mode);
        Assert.Equal(Prime95Mode.Avx2, profile.Phases[1].Mode);
    }
}
