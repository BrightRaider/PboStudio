using PboStudio.Core;
using PboStudio.Tests.Mocks;
using Xunit;

namespace PboStudio.Tests;

public class TestRunnerOrderTests
{
    [Fact]
    public void MockSmuService_ReadAndWrite_WorksAsExpected()
    {
        using var smu = new MockSmuService();

        Assert.True(smu.IsAvailable);
        Assert.Equal(8, smu.PhysicalCores);

        smu.WriteCurveOptimizer(0, -25);
        smu.WriteCurveOptimizer(1, -30);

        Assert.Equal(-25, smu.ReadCurveOptimizer(0));
        Assert.Equal(-30, smu.ReadCurveOptimizer(1));
        Assert.Equal(0, smu.ReadCurveOptimizer(2));

        var pbo = smu.ReadPbo();
        Assert.NotNull(pbo);
        Assert.True(pbo.BoostMhz > 0);
    }

    [Fact]
    public void SmuService_ReadsAllZero_CorrectlyDetectsAllZeroes()
    {
        using var smu = new MockSmuService();
        Assert.True(smu.ReadsAllZero());

        smu.WriteCurveOptimizer(3, -15);
        Assert.False(smu.ReadsAllZero());
    }
}
