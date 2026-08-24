using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

/// <summary>
/// Chiplet grouping used to be <c>index / (coreCount / 2)</c>, which is right only for a
/// two-CCD desktop part. These cases pin the real processors it has to handle.
/// </summary>
public class CoreTopologyCcdTests
{
    /// <summary>Builds cores from an explicit per-core last-level-cache group assignment.</summary>
    private static IReadOnlyList<PhysicalCore> Cores(int[] l3PerCore, int[]? diePerCore = null)
    {
        var list = new List<PhysicalCore>();
        for (int i = 0; i < l3PerCore.Length; i++)
        {
            nuint mask = (nuint)3 << (i * 2);
            list.Add(new PhysicalCore(i, mask, [i * 2, i * 2 + 1], l3PerCore[i], diePerCore?[i] ?? 0));
        }
        return list;
    }

    /// <summary>`groups` cache groups of `coresPerGroup` cores each, laid out consecutively.</summary>
    private static int[] Layout(int groups, int coresPerGroup) =>
        [.. Enumerable.Range(0, groups * coresPerGroup).Select(i => i / coresPerGroup)];

    [Fact]
    public void SingleCcx_IsOneCcd()
    {
        // 5800X / 7800X3D: eight cores on one L3 slice.
        var layout = CoreTopology.ResolveCcds(Cores(Layout(1, 8)));

        Assert.Equal(1, layout.CcdCount);
        Assert.False(layout.IsMultiCcd);
        Assert.All(Enumerable.Range(0, 8), i => Assert.Equal(0, layout.CcdFor(i)));
    }

    [Theory]
    [InlineData(2, 8, 2)]  // 5950X / 7950X / 9950X: two CCDs of eight
    [InlineData(2, 6, 2)]  // 5900X / 7900X: two CCDs of six
    [InlineData(8, 8, 8)]  // Threadripper 7980X: eight CCDs of eight
    [InlineData(4, 8, 4)]  // Threadripper 7970X: four CCDs of eight
    public void ZenThreePlus_OneCcxPerCcd(int groups, int coresPerGroup, int expectedCcds)
    {
        var layout = CoreTopology.ResolveCcds(Cores(Layout(groups, coresPerGroup)));

        Assert.Equal(expectedCcds, layout.CcdCount);
        Assert.Equal(Enumerable.Range(0, coresPerGroup), layout.CoresOn(0));
        Assert.Equal(coresPerGroup, layout.CoresOn(expectedCcds - 1).Count());
    }

    [Theory]
    [InlineData(2, 4, 1)]   // 3700X: two four-core CCXs share a single CCD
    [InlineData(2, 3, 1)]   // 3600: two three-core CCXs, still one CCD
    [InlineData(4, 4, 2)]   // 3950X: four CCXs across two CCDs
    [InlineData(4, 3, 2)]   // 3900X: four three-core CCXs across two CCDs
    [InlineData(16, 4, 8)]  // Threadripper 3990X: sixteen CCXs across eight CCDs
    public void ZenAndZenTwo_TwoCcxPerCcd_WithoutFuse(int ccxGroups, int coresPerCcx, int expectedCcds)
    {
        // No driver, so no fuse reading: the CCX size has to carry the decision.
        var layout = CoreTopology.ResolveCcds(Cores(Layout(ccxGroups, coresPerCcx)), smuCcdCount: null);

        Assert.Equal(expectedCcds, layout.CcdCount);
        Assert.Equal(ccxGroups * coresPerCcx / expectedCcds, layout.CoresOn(0).Count());
    }

    [Fact]
    public void Fuse_OverridesCacheGrouping()
    {
        // Four CCXs the OS can see, two CCDs the processor reports.
        var layout = CoreTopology.ResolveCcds(Cores(Layout(4, 4)), smuCcdCount: 2);

        Assert.Equal(2, layout.CcdCount);
        Assert.Equal([0, 1, 2, 3, 4, 5, 6, 7], layout.CoresOn(0));
        Assert.Equal([8, 9, 10, 11, 12, 13, 14, 15], layout.CoresOn(1));
    }

    [Fact]
    public void Fuse_MatchingCacheGrouping_IsLeftAlone()
    {
        var layout = CoreTopology.ResolveCcds(Cores(Layout(2, 8)), smuCcdCount: 2);

        Assert.Equal(2, layout.CcdCount);
        Assert.Equal(0, layout.CcdFor(7));
        Assert.Equal(1, layout.CcdFor(8));
    }

    [Fact]
    public void ExplicitDies_WinOverCacheGrouping()
    {
        // Windows enumerating separate dies is the most direct evidence there is, so it is
        // used even when the cache view would have split things up differently.
        var l3 = Layout(4, 4);
        var dies = Enumerable.Range(0, 16).Select(i => i < 8 ? 0 : 1).ToArray();

        var layout = CoreTopology.ResolveCcds(Cores(l3, dies));

        Assert.Equal(2, layout.CcdCount);
        Assert.Equal([0, 1, 2, 3, 4, 5, 6, 7], layout.CoresOn(0));
    }

    [Fact]
    public void OddCcxCount_IsNeverMerged()
    {
        // Three CCXs cannot be two-per-CCD; merging would silently lose a core group.
        var layout = CoreTopology.ResolveCcds(Cores(Layout(3, 4)), smuCcdCount: null);

        Assert.Equal(3, layout.CcdCount);
        Assert.Equal(12, Enumerable.Range(0, 3).Sum(c => layout.CoresOn(c).Count()));
    }

    [Fact]
    public void EveryCoreIsAssignedExactlyOnce()
    {
        var layout = CoreTopology.ResolveCcds(Cores(Layout(8, 8)));

        var assigned = Enumerable.Range(0, layout.CcdCount).SelectMany(layout.CoresOn).ToList();
        Assert.Equal(64, assigned.Count);
        Assert.Equal(64, assigned.Distinct().Count());
    }

    [Fact]
    public void NoCores_DoesNotThrow()
    {
        var layout = CoreTopology.ResolveCcds([]);

        Assert.Equal(1, layout.CcdCount);
        Assert.Empty(layout.CoreToCcd);
    }
}
