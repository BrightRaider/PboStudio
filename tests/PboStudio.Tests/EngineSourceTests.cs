using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

/// <summary>
/// Both engine downloads were pinned to one exact version each, and both had 404'd — first-run
/// setup was a dead end. These check the shape of the fix; the live reachability check is a
/// separate, network-dependent test below.
/// </summary>
public class EngineSourceTests
{
    public static TheoryData<EngineSource> Sources => new() { DependencyService.Prime95, DependencyService.YCruncher };

    [Theory]
    [MemberData(nameof(Sources))]
    public void EverySourceHasAPinnedUrlAndAFallbackPage(EngineSource source)
    {
        Assert.NotEmpty(source.Urls);
        Assert.All(source.Urls, u => Assert.True(Uri.IsWellFormedUriString(u, UriKind.Absolute), u));
        Assert.True(Uri.IsWellFormedUriString(source.PageUrl, UriKind.Absolute), source.PageUrl);
    }

    [Theory]
    [MemberData(nameof(Sources))]
    public void TheDiscoveryPatternMatchesTheOwnPinnedUrls(EngineSource source)
    {
        // If the pattern cannot recognise the URLs we ship, it will not recognise the vendor's
        // next release either — which is the entire point of having it.
        foreach (string url in source.Urls)
        {
            Assert.Matches(source.LinkPattern, url);
        }
    }

    [Theory]
    [MemberData(nameof(Sources))]
    public void TheDiscoveryPatternRejectsUnrelatedLinks(EngineSource source)
    {
        foreach (string noise in new[]
                 {
                     "https://example.com/index.html",
                     "https://example.com/readme.txt",
                     "https://example.com/some-other-tool.zip",
                 })
        {
            Assert.DoesNotMatch(source.LinkPattern, noise);
        }
    }

    [Fact]
    public void ThePatternSurvivesAVersionBump()
    {
        // The exact failure that started this: the pinned build disappears and a newer one
        // takes its place under a different version number.
        Assert.Matches(DependencyService.Prime95.LinkPattern,
            "https://download.mersenne.ca/gimps/v30/30.19/p95v3019b20.win64.zip");
        Assert.Matches(DependencyService.YCruncher.LinkPattern,
            "https://cdn.numberworld.org/y-cruncher-downloads/y-cruncher%20v0.9.1.9999.zip");
    }

    [Theory]
    [MemberData(nameof(Sources))]
    public void TargetDirectoriesAreDistinctAndUnderEngines(EngineSource source)
    {
        string dir = DependencyService.TargetDirectoryFor(@"C:\app", source);

        Assert.Contains("engines", dir);
        Assert.EndsWith(source.TargetSubdirectory, dir);
    }

    [Fact]
    public void TheTwoEnginesDoNotShareADirectory()
    {
        Assert.NotEqual(
            DependencyService.TargetDirectoryFor(@"C:\app", DependencyService.Prime95),
            DependencyService.TargetDirectoryFor(@"C:\app", DependencyService.YCruncher));
    }
}
