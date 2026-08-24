using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

public class StressKernelTests
{
    [Fact]
    public void StressKernel_RunBlock_IsBitDeterministic()
    {
        var kernel1 = new StressKernel();
        ulong checksum1 = kernel1.RunBlock(100);

        var kernel2 = new StressKernel();
        ulong checksum2 = kernel2.RunBlock(100);

        Assert.Equal(checksum1, checksum2);
        Assert.NotEqual(0UL, checksum1);
    }

    [Fact]
    public void StressKernel_Reset_RestoresState()
    {
        var kernel = new StressKernel();
        ulong checksumA = kernel.RunBlock(50);

        kernel.Reset();
        ulong checksumB = kernel.RunBlock(50);

        Assert.Equal(checksumA, checksumB);
    }
}
