using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

public class CpuRecommendationServiceTests
{
    private static IReadOnlyList<PhysicalCore> CreateMockCores(int count)
    {
        var list = new List<PhysicalCore>();
        for (int i = 0; i < count; i++)
        {
            nuint mask = (nuint)3 << (i * 2);
            list.Add(new PhysicalCore(i, mask, [i * 2, i * 2 + 1]));
        }
        return list;
    }

    [Theory]
    [InlineData("AMD Ryzen 7 7800X3D 8-Core Processor", ZenGeneration.Zen4, true, -15)]
    [InlineData("AMD Ryzen 9 7950X 16-Core Processor", ZenGeneration.Zen4, false, -22)]
    [InlineData("AMD Ryzen 7 9800X3D 8-Core Processor", ZenGeneration.Zen5, true, -15)]
    [InlineData("AMD Ryzen 9 9950X 16-Core Processor", ZenGeneration.Zen5, false, -20)]
    [InlineData("AMD Ryzen 7 5800X3D 8-Core Processor", ZenGeneration.Zen3, true, -10)]
    [InlineData("AMD Ryzen 5 5600X 6-Core Processor", ZenGeneration.Zen3, false, -18)]
    public void GetRecommendation_DetectsGenerationAndBaseCoCorrectly(
        string cpuName, ZenGeneration expectedGen, bool expectedX3D, int expectedCo)
    {
        var cores = CreateMockCores(8);
        var rec = CpuRecommendationService.GetRecommendation(cpuName, cores);

        Assert.Equal(expectedGen, rec.Generation);
        Assert.Equal(expectedX3D, rec.IsX3D);
        Assert.Equal(expectedCo, rec.DefaultStartingCo);
    }

    [Fact]
    public void GetRecommendation_WithWheaEvents_CalculatesBackoff()
    {
        var cores = CreateMockCores(8);
        var wheaEvents = new List<WheaEvent>
        {
            new(19, DateTime.Now, true, "Processor APIC ID: 2", ApicId: 2), // Core 1 (logical 2,3)
            new(19, DateTime.Now, true, "Processor APIC ID: 2", ApicId: 2), // Core 1 again
        };

        var currentMargins = new Dictionary<int, int>
        {
            [0] = -25,
            [1] = -25,
            [2] = -25,
        };

        // Grob mode: count(2) * 4 = +8 backoff -> -25 + 8 = -17
        var recGrob = CpuRecommendationService.GetRecommendation(
            "AMD Ryzen 7 7800X3D", cores, wheaEvents, currentMargins, isFineMode: false);

        Assert.Equal(-17, recGrob.SuggestedCoreMargins[1]);
        Assert.Single(recGrob.WheaCorrections);

        // Fine mode: count(2) * 2 = +4 backoff -> -25 + 4 = -21
        var recFine = CpuRecommendationService.GetRecommendation(
            "AMD Ryzen 7 7800X3D", cores, wheaEvents, currentMargins, isFineMode: true);

        Assert.Equal(-21, recFine.SuggestedCoreMargins[1]);
    }
}
