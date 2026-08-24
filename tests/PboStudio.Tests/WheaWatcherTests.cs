using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

public class WheaWatcherTests
{
    [Theory]
    [InlineData("A corrected hardware error has occurred. Component: Processor Core. Error Source: Corrected Machine Check. Processor APIC ID: 4", 4)]
    [InlineData("Ein korrigierter Hardwarefehler ist aufgetreten. Prozessor-APIC-ID: 12", 12)]
    [InlineData("WHEA Logger event: APIC ID: 0", 0)]
    [InlineData("WHEA-Logger: APIC-ID: 8", 8)]
    [InlineData("Unrelated event without APIC identifier", null)]
    public void ParseApicId_ParsesVariousLanguageFormats(string description, int? expectedId)
    {
        var apicId = WheaEvent.ParseApicId(description);
        Assert.Equal(expectedId, apicId);
    }

    [Fact]
    public void CoreForApicId_MapsLogicalProcessorToPhysicalCore()
    {
        var cores = new List<PhysicalCore>
        {
            new(0, 0x3, [0, 1]),
            new(1, 0xC, [2, 3]),
            new(2, 0x30, [4, 5]),
            new(3, 0xC0, [6, 7]),
        };

        Assert.Equal(0, WheaWatcher.CoreForApicId(0, cores));
        Assert.Equal(0, WheaWatcher.CoreForApicId(1, cores));
        Assert.Equal(1, WheaWatcher.CoreForApicId(2, cores));
        Assert.Equal(1, WheaWatcher.CoreForApicId(3, cores));
        Assert.Equal(2, WheaWatcher.CoreForApicId(4, cores));
        Assert.Equal(3, WheaWatcher.CoreForApicId(7, cores));
        Assert.Null(WheaWatcher.CoreForApicId(99, cores));
    }
}
