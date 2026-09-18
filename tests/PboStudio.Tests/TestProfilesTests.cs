using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

public class TestProfilesTests
{
    private static IReadOnlyList<TestProfile> Profiles(string cpu = "AMD Ryzen 7 7700X", bool de = false) =>
        TestProfiles.For(cpu, isGerman: de);

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void EveryProfileIsSelfContained(bool german)
    {
        // Picking a profile has to be the whole configuration - that is the point of having
        // them. A profile missing its engine, runtime or pause values would force the user
        // back into the Engine tab.
        foreach (var p in Profiles(de: german))
        {
            Assert.False(string.IsNullOrWhiteSpace(p.Name), "profile without a name");
            Assert.False(string.IsNullOrWhiteSpace(p.Explanation), p.Name);
            Assert.NotEmpty(p.Phases);
            Assert.NotEmpty(p.Engines);
            Assert.InRange(p.MinutesPerCore, 1, 600);
            Assert.InRange(p.Iterations, 1, 99);
            Assert.InRange(p.Threads, 1, 2);
            Assert.InRange(p.SuspendEverySeconds, 0, 300);
            Assert.InRange(p.SuspendForSeconds, 1, 30);
        }
    }

    [Fact]
    public void TheLineupCoversBothEnginesOnItsOwn()
    {
        var profiles = Profiles();

        Assert.Contains(profiles, p => p.Engines.Count == 1 && p.Uses(EngineKind.Prime95));
        Assert.Contains(profiles, p => p.Engines.Count == 1 && p.Uses(EngineKind.YCruncher));
        Assert.Contains(profiles, p => p.Uses(EngineKind.Prime95) && p.Uses(EngineKind.YCruncher));
    }

    [Fact]
    public void ThereIsAProfileBuiltForTransientStress()
    {
        // The load transition is what breaks a borderline value; without a profile carrying it,
        // the setting has to be dialled in by hand every time.
        Assert.Contains(Profiles(), p => p.SuspendEverySeconds <= 10 && p.Threads == 2);
    }

    [Fact]
    public void ProfileNamesAreUnique()
    {
        var names = Profiles().Select(p => p.Name).ToList();
        Assert.Equal(names.Count, names.Distinct().Count());
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void EngineSummaryNamesEveryPhase(bool german)
    {
        foreach (var p in Profiles(de: german))
        {
            string summary = p.EngineSummary;

            foreach (var phase in p.Phases)
            {
                string expected = phase.Engine == EngineKind.YCruncher ? "y-cruncher" : "Prime95";
                Assert.Contains(expected, summary);
            }

            Assert.Contains($"{p.MinutesPerCore}", summary);
        }
    }

    [Fact]
    public void X3dAddsThermalAdviceToTheFullPrime95Run()
    {
        foreach (bool german in new[] { false, true })
        {
            var x3d = TestProfiles.For("AMD Ryzen 7 7800X3D", isGerman: german);
            Assert.Contains(x3d, p => p.Explanation.Contains("3D V-Cache"));

            var plain = TestProfiles.For("AMD Ryzen 7 7700X", isGerman: german);
            Assert.DoesNotContain(plain, p => p.Explanation.Contains("3D V-Cache"));
        }
    }

    [Fact]
    public void Phases_ChainsBaseAndExtraPhases()
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

    [Theory]
    [InlineData(FftPreset.Smallest, 4, 21)]
    [InlineData(FftPreset.Heavy, 4, 1344)]
    [InlineData(FftPreset.HeavyShort, 4, 160)]
    [InlineData(FftPreset.Moderate, 1344, 4096)]
    [InlineData(FftPreset.Huge, 8960, 32768)]
    [InlineData(FftPreset.All, 4, 32768)]
    public void FftRangesMatchCoreCycler(FftPreset preset, int min, int max)
    {
        var options = new Prime95Options(Prime95Mode.Sse, preset);
        Assert.Equal((min, max), options.FftRange);
    }
}

/// <summary>
/// The short list. Fourteen profiles in one dropdown is not a choice, it is a quiz: nine of
/// them are variations on the five that actually get somebody from an untested processor to
/// values worth putting in the BIOS. The picker shows five and keeps the rest behind a
/// checkbox, which only works as long as the five really are the path.
/// </summary>
public class EssentialProfileTests
{
    private const string Cpu = "AMD Ryzen 7 5800X3D 8-Core Processor";

    private static IReadOnlyList<TestProfile> Profiles => TestProfiles.For(Cpu, isGerman: false);

    [Fact]
    public void TheShortListIsShortEnoughToReadWithoutScrolling()
    {
        Assert.Equal(5, Profiles.Count(p => p.Essential));
    }

    /// <summary>
    /// The coupling that would break silently: the card recommends a profile by id and the
    /// picker then has to be able to show it. If a recommendable profile were left off the
    /// short list, selecting it would widen the list behind the user's back.
    /// </summary>
    [Fact]
    public void EveryProfileTheProgramRecommendsIsOnTheShortList()
    {
        var recommended = new[]
        {
            ProfileId.RecommendedCombo,   // Discover
            ProfileId.HeavyFfts,          // Narrow
            ProfileId.Overnight,          // Confirm
        };

        foreach (var id in recommended)
            Assert.True(Profiles.Single(p => p.Id == id).Essential, $"{id} must be on the short list");
    }

    /// <summary>
    /// The one case a stress test cannot produce on its own, and the reason this machine
    /// crashed in a game after passing everything else. It has to be findable.
    /// </summary>
    [Fact]
    public void TheMicroBurstProfileIsOnTheShortList()
    {
        Assert.True(Profiles.Single(p => p.Id == ProfileId.MicroBurst).Essential);
    }

    [Fact]
    public void TheShortListCoversBothEnginesAndBothEndsOfTheRuntimeRange()
    {
        var shown = Profiles.Where(p => p.Essential).ToList();

        Assert.Contains(shown, p => p.Uses(EngineKind.YCruncher));
        Assert.Contains(shown, p => p.Uses(EngineKind.Prime95));

        // Something short enough for an afternoon, and something long enough to trust.
        Assert.Contains(shown, p => p.MinutesPerCore * p.Iterations <= 6);
        Assert.Contains(shown, p => p.Phases.Count >= 4);
    }

    /// <summary>The nine that stay behind the checkbox are still there, not deleted.</summary>
    [Fact]
    public void NothingWasRemovedFromTheFullList()
    {
        Assert.Equal(Enum.GetValues<ProfileId>().Length, Profiles.Count);
    }
}
