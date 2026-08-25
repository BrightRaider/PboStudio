using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

/// <summary>
/// The algorithm tags go straight onto y-cruncher's command line, so a wrong spelling does not
/// fail loudly - it just asks for a test that does not exist.
///
/// y-cruncher 0.8.x renamed "SFT" to "SFTv4" and "FFT" to "FFTv4". Its own "Command Lines.txt"
/// lists the surviving aliases explicitly (N64 to NTT63, VST to VSTv3) and those two are not
/// among them, so every preset shipped here used two tags the binary no longer knows.
/// </summary>
public class YCruncherAlgorithmTests
{
    /// <summary>The exact set y-cruncher 0.8.7 documents for the "stress" verb.</summary>
    private static readonly HashSet<string> Valid =
        ["BKT", "BBP", "SFTv4", "SNT", "SVT", "FFTv4", "NTT63", "N63", "VSTv3", "VT3"];

    public static TheoryData<string, IReadOnlyList<string>> Presets => new()
    {
        { nameof(YCruncherOptions.Default), YCruncherOptions.Default.Algorithms },
        { nameof(YCruncherOptions.CurveOptimizer), YCruncherOptions.CurveOptimizer.Algorithms },
        { nameof(YCruncherOptions.FastDiscovery), YCruncherOptions.FastDiscovery.Algorithms },
    };

    [Theory]
    [MemberData(nameof(Presets))]
    public void EveryPresetTagIsAcceptedByYCruncher(string name, IReadOnlyList<string> algorithms)
    {
        Assert.NotEmpty(algorithms);

        var unknown = algorithms.Where(a => !Valid.Contains(a)).ToList();
        Assert.True(unknown.Count == 0,
            $"{name} passes tags y-cruncher 0.8.x does not accept: {string.Join(", ", unknown)}");
    }

    [Fact]
    public void RetiredV07SpellingsAreGone()
    {
        var all = YCruncherOptions.Default.Algorithms
            .Concat(YCruncherOptions.CurveOptimizer.Algorithms)
            .Concat(YCruncherOptions.FastDiscovery.Algorithms)
            .ToList();

        Assert.DoesNotContain("SFT", all);
        Assert.DoesNotContain("FFT", all);
    }

    [Fact]
    public void AllAlgorithmsIsTheListTheUiOffers()
    {
        // The checkbox list is generated from this, so an invalid entry here would put a tag
        // in front of the user that the command line then rejects.
        Assert.All(YCruncherOptions.AllAlgorithms, tag => Assert.Contains(tag, Valid));
        Assert.Equal(8, YCruncherOptions.AllAlgorithms.Count);
    }

    [Fact]
    public void CurveOptimizerPresetIsASubsetOfTheFullList()
    {
        Assert.All(YCruncherOptions.CurveOptimizer.Algorithms,
            tag => Assert.Contains(tag, YCruncherOptions.AllAlgorithms));
    }

    [Fact]
    public void SelectableBinariesAreDistinctAndPrefixed()
    {
        var prefixes = YCruncherEngine.SelectableBinaries.Select(b => b.Prefix).ToList();
        Assert.Equal(prefixes.Count, prefixes.Distinct().Count());
        Assert.All(prefixes, p => Assert.Matches(@"^\d\d-", p));
    }

    [Fact]
    public void FindBinaryFor_ReturnsNullWhenThereIsNoBinariesFolder()
    {
        string empty = Path.Combine(Path.GetTempPath(), "pbostudio-yc-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(empty);
        try
        {
            Assert.Null(YCruncherEngine.FindBinaryFor(empty, ZenGeneration.Zen4));
            Assert.Empty(YCruncherEngine.AvailableBinaries(empty));
        }
        finally
        {
            Directory.Delete(empty, recursive: true);
        }
    }

    [Fact]
    public void FindBinaryFor_PrefersTheExplicitPrefixButFallsBackWhenItIsMissing()
    {
        string root = Path.Combine(Path.GetTempPath(), "pbostudio-yc-" + Guid.NewGuid().ToString("N"));
        string binaries = Path.Combine(root, "Binaries");
        Directory.CreateDirectory(binaries);
        try
        {
            File.WriteAllText(Path.Combine(binaries, "00-x86.exe"), "");
            File.WriteAllText(Path.Combine(binaries, "22-ZN4 ~ Kizuna.exe"), "");

            string? chosen = YCruncherEngine.FindBinaryFor(root, ZenGeneration.Zen4, "00-x86");
            Assert.EndsWith("00-x86.exe", chosen);

            // Auto-detection still wins when the requested build is not installed, because an
            // installation that lacks it has to remain runnable.
            string? fallback = YCruncherEngine.FindBinaryFor(root, ZenGeneration.Zen4, "24-ZN5");
            Assert.EndsWith("22-ZN4 ~ Kizuna.exe", fallback);

            Assert.Equal(2, YCruncherEngine.AvailableBinaries(root).Count);
        }
        finally
        {
            Directory.Delete(root, recursive: true);
        }
    }
}
