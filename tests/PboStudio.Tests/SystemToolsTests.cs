using System.Diagnostics;
using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

/// <summary>
/// The small system-facing pieces: starting Windows tools, deciding what of a saved profile may be
/// written back at logon, what the platform audit may claim, and what ends up in the report.
/// </summary>
public class SystemToolsTests
{
    // -- starting tools -----------------------------------------------------------------------

    /// <summary>
    /// Started by bare name, a tool is looked for in the program's own folder first. The program
    /// runs as administrator; whatever sits next to it with the right name would run with those
    /// rights.
    /// </summary>
    [Fact]
    public void TheToolsAreStartedByTheirFullPathInTheSystemFolder()
    {
        foreach (string tool in new[] { SystemTools.PowerShell, SystemTools.SchTasks, SystemTools.Shutdown, SystemTools.RunDll32 })
        {
            Assert.True(Path.IsPathRooted(tool), $"{tool} is a bare name");
            Assert.StartsWith(Environment.GetFolderPath(Environment.SpecialFolder.Windows), tool, StringComparison.OrdinalIgnoreCase);
            Assert.True(File.Exists(tool), $"{tool} does not exist");
        }
    }

    [Fact]
    public void ARunReturnsWhatTheProgramPrinted()
    {
        var (ok, output) = ProcessRunner.Run(SystemTools.PowerShell, "-NoProfile -Command \"Write-Output hello\"", TimeSpan.FromSeconds(30));

        Assert.True(ok);
        Assert.Contains("hello", output);
    }

    [Fact]
    public void ANonZeroExitIsAFailureAndTheErrorTextComesBack()
    {
        var (ok, output) = ProcessRunner.Run(
            SystemTools.PowerShell, "-NoProfile -Command \"[Console]::Error.WriteLine('it broke'); exit 3\"", TimeSpan.FromSeconds(30));

        Assert.False(ok);
        Assert.Contains("it broke", output);
    }

    /// <summary>
    /// The bug: standard output was read to the end before the timeout was ever consulted, and
    /// reading to the end blocks until the process lets go of its output. A hung PowerShell froze
    /// the caller for as long as it hung - the thread that was trying to start a run.
    /// </summary>
    [Fact]
    public void AProcessThatHangsIsGivenUpOnAtTheTimeoutNotWhenItFinishes()
    {
        var watch = Stopwatch.StartNew();

        var (ok, output) = ProcessRunner.Run(
            SystemTools.PowerShell, "-NoProfile -Command \"Start-Sleep -Seconds 40\"", TimeSpan.FromSeconds(3));

        watch.Stop();

        Assert.False(ok);
        Assert.Equal("timed out", output);
        Assert.True(watch.Elapsed < TimeSpan.FromSeconds(20), $"waited {watch.Elapsed} for a 3 s timeout");
    }

    [Fact]
    public void AProgramThatDoesNotExistFailsInsteadOfThrowing()
    {
        var (ok, _) = ProcessRunner.Run(@"C:\definitely\not\here.exe", "", TimeSpan.FromSeconds(5));

        Assert.False(ok);
    }

    // -- the saved profile written back at logon ----------------------------------------------

    private const string Cpu = "AMD Ryzen 7 5800X3D 8-Core Processor";

    private static SavedCoProfile Profile(string cpu, params (int core, int value)[] margins) =>
        new(DateTime.Now, cpu, margins.ToDictionary(m => m.core, m => m.value));

    [Fact]
    public void AProfileForThisProcessorIsUsable()
    {
        var check = SavedProfileValidator.Check(Profile(Cpu, (0, -20), (1, -25), (7, -30)), Cpu, 8, -30);

        Assert.False(check.WrongProcessor);
        Assert.Empty(check.Problems);
        Assert.Equal(3, check.Margins.Count);
    }

    /// <summary>
    /// Swapping a 5800X3D for a 7800X3D leaves eight cores and an eight-entry file. Every old value
    /// would have been written to a chip it was never tested on.
    /// </summary>
    [Fact]
    public void AProfileSavedOnAnotherProcessorIsNotApplied()
    {
        var check = SavedProfileValidator.Check(
            Profile("AMD Ryzen 7 7800X3D 8-Core Processor", (0, -20), (1, -25)), Cpu, 8, -30);

        Assert.True(check.WrongProcessor);
        Assert.Empty(check.Margins);
        Assert.Contains("7800X3D", check.Problems[0]);
    }

    [Fact]
    public void TheProcessorNameIsComparedWithoutCaseOrPadding()
    {
        var check = SavedProfileValidator.Check(Profile("  " + Cpu.ToUpperInvariant() + " ", (0, -20)), Cpu, 8, -30);

        Assert.False(check.WrongProcessor);
        Assert.Single(check.Margins);
    }

    /// <summary>A damaged file, or a figure no processor accepts, went to the SMU as it stood.</summary>
    [Fact]
    public void ValuesOutsideTheRangeAreLeftOutAndNamed()
    {
        var check = SavedProfileValidator.Check(
            Profile(Cpu, (0, -20), (1, -200), (2, 99), (3, -30)), Cpu, 8, -30);

        Assert.Equal([0, 3], check.Margins.Keys.Order());
        Assert.Equal(2, check.Problems.Count);
    }

    [Fact]
    public void ACoreThatDoesNotExistIsLeftOut()
    {
        var check = SavedProfileValidator.Check(Profile(Cpu, (0, -20), (12, -20), (-1, -5)), Cpu, 8, -30);

        Assert.Equal([0], check.Margins.Keys);
        Assert.Equal(2, check.Problems.Count);
    }

    /// <summary>The allowed depth follows the processor: -45 is fine on a part that goes there.</summary>
    [Fact]
    public void TheAllowedDepthIsWhateverTheCallerSays()
    {
        var profile = Profile("AMD Ryzen 7 9700X 8-Core Processor", (0, -45));

        Assert.Empty(SavedProfileValidator.Check(profile, profile.CpuName, 8, -50).Margins.Keys.Except([0]));
        Assert.Empty(SavedProfileValidator.Check(profile, profile.CpuName, 8, -30).Margins);
    }

    // -- the platform audit --------------------------------------------------------------------

    /// <summary>
    /// When WMI could not be read the audit substituted 4800 and 6000, which reads as "EXPO is off"
    /// and cost 25 points for a fault that was never measured.
    /// </summary>
    [Fact]
    public void AFailedReadingIsUnknownNotAFinding()
    {
        var ram = BiosAuditService.ClassifyRam(0, 0, readFailed: true);

        Assert.False(ram.Known);
        Assert.True(ram.IsXmpExpoActive);   // no penalty for a reading that did not happen
    }

    [Fact]
    public void AnEmptyReadingIsAlsoUnknown()
    {
        Assert.False(BiosAuditService.ClassifyRam(0, 0, readFailed: false).Known);
    }

    [Theory]
    [InlineData(3600, 3600, true)]    // the DDR4 EXPO/XMP case this program was verified on
    [InlineData(6000, 6000, true)]
    [InlineData(2133, 3200, false)]   // running at the JEDEC base speed
    [InlineData(4800, 6000, false)]
    public void AKnownReadingIsStillJudgedAsBefore(int configured, int max, bool active)
    {
        var ram = BiosAuditService.ClassifyRam(configured, max, readFailed: false);

        Assert.True(ram.Known);
        Assert.Equal(active, ram.IsXmpExpoActive);
    }

    // -- the report ----------------------------------------------------------------------------

    private static string Report(string cpu, string profile) => ReportExporterService.GenerateHtmlReport(
        cpu, new Dictionary<int, int> { [0] = -20 }, [], profile, TimeSpan.FromHours(26), 0);

    /// <summary>The processor and profile names were written into the page as markup.</summary>
    [Fact]
    public void TextThatIsNotOursIsEncodedBeforeItIsWrittenIntoThePage()
    {
        string html = Report("<script>alert(1)</script>", "<img src=x onerror=1>");

        Assert.DoesNotContain("<script>alert(1)</script>", html);
        Assert.DoesNotContain("<img src=x", html);
        Assert.Contains("&lt;script&gt;", html);
    }

    [Fact]
    public void AnOrdinaryNameIsUnchanged()
    {
        Assert.Contains("AMD Ryzen 7 5800X3D 8-Core Processor", Report(Cpu, "Heavy FFTs"));
    }

    /// <summary>The footer named a version no release has ever had.</summary>
    [Fact]
    public void TheFooterDoesNotClaimAVersionThatNeverExisted()
    {
        Assert.DoesNotContain("v2.0", Report(Cpu, "x"));
    }

    [Fact]
    public void ADurationOverADayIsShownAsHoursNotWrappedToZero()
    {
        Assert.Contains("26:00:00", Report(Cpu, "x"));
    }

    // -- the webhook ---------------------------------------------------------------------------

    /// <summary>
    /// The message carries the processor name and the run's results. A plain-http address is
    /// refused before anything is sent.
    /// </summary>
    [Theory]
    [InlineData("http://example.com/hook")]
    [InlineData("file:///C:/secret.txt")]
    [InlineData("ftp://example.com/hook")]
    [InlineData("not a url")]
    [InlineData("")]
    public async Task OnlyHttpsWebhooksAreEverUsed(string url)
    {
        Assert.False(await NotificationService.SendWebhookAsync(url, "title", "description"));
    }
}
