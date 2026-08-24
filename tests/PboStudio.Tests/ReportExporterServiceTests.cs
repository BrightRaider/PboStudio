using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

public class ReportExporterServiceTests
{
    [Fact]
    public void GenerateHtmlReport_IncludesCpuAndCoreMargins()
    {
        var margins = new Dictionary<int, int>
        {
            [0] = -25,
            [1] = -20,
            [2] = -30,
        };
        var whea = new List<WheaEvent>
        {
            new(19, DateTime.Now, true, "Test error", ApicId: 2)
        };

        string html = ReportExporterService.GenerateHtmlReport(
            "AMD Ryzen 7 7800X3D",
            margins,
            whea,
            "SSE Full Run",
            TimeSpan.FromMinutes(45),
            failureCount: 1
        );

        Assert.Contains("AMD Ryzen 7 7800X3D", html);
        Assert.Contains("SSE Full Run", html);
        Assert.Contains("Kern 0", html);
        Assert.Contains("-25", html);
        Assert.Contains("1 FEHLER ERKANNT", html);
        Assert.Contains("WHEA Event 19", html);
    }
}
