using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

/// <summary>
/// The program runs as administrator and starts what it downloads: two stress engines, and the
/// installer of a kernel driver. These pin down what it will fetch and what it will run.
/// </summary>
public class DownloadPolicyTests
{
    private static readonly string[] YcHosts = ["cdn.numberworld.org", "www.numberworld.org"];
    private static readonly string[] PrimeHosts = ["www.mersenne.org", "download.mersenne.ca"];

    // -- https, and the vendor's own domain --------------------------------------------------

    [Theory]
    [InlineData("https://cdn.numberworld.org/y-cruncher-downloads/y-cruncher%20v0.8.7.9547b.zip")]
    [InlineData("https://www.numberworld.org/y-cruncher/y-cruncher%20v0.8.7.9547b.zip")]
    [InlineData("https://downloads.numberworld.org/x.zip")]   // another host of the same owner
    public void HttpsOnTheVendorsOwnDomainIsAllowed(string url)
    {
        Assert.True(DownloadPolicy.IsAllowed(url, YcHosts));
    }

    /// <summary>A plain-http mirror stood in the list: anything on the path could swap the archive.</summary>
    [Theory]
    [InlineData("http://www.numberworld.org/y-cruncher/y-cruncher%20v0.8.7.9547b.zip")]
    [InlineData("http://cdn.numberworld.org/x.zip")]
    [InlineData("ftp://www.numberworld.org/x.zip")]
    public void AnythingNotHttpsIsRefusedEvenOnTheRightDomain(string url)
    {
        Assert.False(DownloadPolicy.IsAllowed(url, YcHosts));
    }

    /// <summary>A link found on a page is not trusted because the page named it.</summary>
    [Theory]
    [InlineData("https://evil.example.com/y-cruncher.zip")]
    [InlineData("https://numberworld.org.evil.com/y-cruncher.zip")]   // the name as a prefix
    [InlineData("https://evilnumberworld.org/y-cruncher.zip")]        // the name as a suffix of another word
    [InlineData("https://www.mersenne.org/y-cruncher.zip")]           // a different vendor
    public void ALinkOnSomebodyElsesDomainIsRefused(string url)
    {
        Assert.False(DownloadPolicy.IsAllowed(url, YcHosts));
    }

    [Fact]
    public void EachVendorsDomainIsItsOwn()
    {
        Assert.True(DownloadPolicy.IsAllowed("https://download.mersenne.ca/gimps/v30/30.8/p95v308b17.win64.zip", PrimeHosts));
        Assert.False(DownloadPolicy.IsAllowed("https://cdn.numberworld.org/x.zip", PrimeHosts));
    }

    [Theory]
    [InlineData("not a url")]
    [InlineData("")]
    [InlineData("//www.numberworld.org/x.zip")]
    public void SomethingThatIsNotAnAbsoluteUrlIsRefused(string url)
    {
        Assert.False(DownloadPolicy.IsAllowed(url, YcHosts));
    }

    [Theory]
    [InlineData("cdn.numberworld.org", "numberworld.org")]
    [InlineData("numberworld.org", "numberworld.org")]
    [InlineData("WWW.Mersenne.ORG.", "mersenne.org")]
    [InlineData("localhost", "localhost")]
    public void TheRootIsTheLastTwoLabels(string host, string expected)
    {
        Assert.Equal(expected, DownloadPolicy.RegistrableRoot(host));
    }

    // -- the shipped sources themselves ------------------------------------------------------

    /// <summary>
    /// The mirror lists and the discovery pages must pass their own policy, or the setup would
    /// refuse its own pinned downloads. Also keeps a plain-http address from coming back.
    /// </summary>
    [Theory]
    [InlineData("Prime95")]
    [InlineData("y-cruncher")]
    public void EveryShippedAddressIsHttpsAndPassesItsOwnPolicy(string name)
    {
        var source = name == "Prime95" ? DependencyService.Prime95 : DependencyService.YCruncher;
        var hosts = source.Urls.Append(source.PageUrl).Select(u => new Uri(u).Host).ToList();

        foreach (var url in source.Urls.Append(source.PageUrl))
        {
            Assert.StartsWith("https://", url);
            Assert.True(DownloadPolicy.IsAllowed(url, hosts), url);
        }
    }

    // -- the driver installer ----------------------------------------------------------------

    [Fact]
    public void TheExpectedHashIsAWellFormedSha256()
    {
        Assert.Equal(64, DownloadPolicy.PawnIoInstallerSha256.Length);
        Assert.Matches("^[0-9a-f]{64}$", DownloadPolicy.PawnIoInstallerSha256);
    }

    [Fact]
    public void AFileMatchesItsOwnHashAndNothingElse()
    {
        string path = Path.GetTempFileName();
        try
        {
            File.WriteAllText(path, "abc");

            // SHA-256("abc"), the standard test vector.
            const string abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

            Assert.True(DownloadPolicy.Sha256Matches(path, abc));
            Assert.True(DownloadPolicy.Sha256Matches(path, abc.ToUpperInvariant()));
            Assert.False(DownloadPolicy.Sha256Matches(path, DownloadPolicy.PawnIoInstallerSha256));
        }
        finally
        {
            File.Delete(path);
        }
    }

    /// <summary>One flipped byte is enough to refuse the installer.</summary>
    [Fact]
    public void ATamperedFileDoesNotMatch()
    {
        string path = Path.GetTempFileName();
        try
        {
            File.WriteAllBytes(path, [1, 2, 3, 4]);
            string good = Convert.ToHexString(System.Security.Cryptography.SHA256.HashData([1, 2, 3, 4]));
            Assert.True(DownloadPolicy.Sha256Matches(path, good));

            File.WriteAllBytes(path, [1, 2, 3, 5]);
            Assert.False(DownloadPolicy.Sha256Matches(path, good));
        }
        finally
        {
            File.Delete(path);
        }
    }

    /// <summary>
    /// The copy shipped in the offline bundle has to match the pinned hash, or the bundled path would
    /// refuse its own installer. Skipped where the bundle has not been assembled.
    /// </summary>
    [Fact]
    public void TheBundledInstallerMatchesThePinnedHash()
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir is not null && !File.Exists(Path.Combine(dir.FullName, "PboStudio.sln"))) dir = dir.Parent;
        if (dir is null) return;

        string bundled = Path.Combine(dir.FullName, "release", "full", "drivers", "PawnIO_setup.exe");
        if (!File.Exists(bundled)) return;

        Assert.True(DownloadPolicy.Sha256Matches(bundled, DownloadPolicy.PawnIoInstallerSha256));
    }

    [Fact]
    public void AMissingFileDoesNotMatchRatherThanThrowing()
    {
        Assert.False(DownloadPolicy.Sha256Matches(Path.Combine(Path.GetTempPath(), "absent-" + Guid.NewGuid().ToString("N")),
            DownloadPolicy.PawnIoInstallerSha256));
    }
}
