using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

/// <summary>
/// The badges used to be hard-coded (core 0 gold, core 8 silver) and shown as if measured.
/// These pin the rule that a badge only ever appears when the processor actually said so.
/// </summary>
public class CoreQualityServiceTests
{
    private static CoreQualityRank RankOf(Dictionary<int, CoreQualityInfo> q, int core) => q[core].Rank;

    [Fact]
    public void WithoutReadings_NoCoreGetsABadge()
    {
        var q = CoreQualityService.GetCoreQualities(8, cppcPerformance: null);

        Assert.Equal(8, q.Count);
        Assert.All(q.Values, v => Assert.Equal(CoreQualityRank.Standard, v.Rank));
    }

    [Fact]
    public void AllZeroReadings_MeanFailedReads_NotLowScores()
    {
        var q = CoreQualityService.GetCoreQualities(8, [0, 0, 0, 0, 0, 0, 0, 0]);

        Assert.All(q.Values, v => Assert.Equal(CoreQualityRank.Standard, v.Rank));
    }

    [Fact]
    public void IdenticalReadings_ProduceNoRanking()
    {
        // Some firmware reports one value for every core; that is not a ranking.
        var q = CoreQualityService.GetCoreQualities(4, [200, 200, 200, 200]);

        Assert.All(q.Values, v => Assert.Equal(CoreQualityRank.Standard, v.Rank));
        Assert.Equal(200, q[0].CppcPerformance);
    }

    [Fact]
    public void HighestValueGetsGold_SecondGetsSilver()
    {
        //            core: 0    1    2    3    4    5    6    7
        var q = CoreQualityService.GetCoreQualities(8, [180, 172, 250, 168, 210, 165, 170, 166]);

        Assert.Equal(CoreQualityRank.GoldStar, RankOf(q, 2));
        Assert.Equal(CoreQualityRank.SilverStar, RankOf(q, 4));
        Assert.Equal(CoreQualityRank.Standard, RankOf(q, 0));
        Assert.Equal(250, q[2].CppcPerformance);
    }

    [Fact]
    public void TheBestCoreIsNotAssumedToBeCoreZero()
    {
        // Exactly the case the old hard-coded heuristic got wrong.
        var q = CoreQualityService.GetCoreQualities(4, [100, 100, 300, 200]);

        Assert.Equal(CoreQualityRank.Standard, RankOf(q, 0));
        Assert.Equal(CoreQualityRank.GoldStar, RankOf(q, 2));
        Assert.Equal(CoreQualityRank.SilverStar, RankOf(q, 3));
    }

    [Fact]
    public void TiesGoToTheLowerCoreIndex()
    {
        var q = CoreQualityService.GetCoreQualities(4, [250, 250, 100, 100]);

        Assert.Equal(CoreQualityRank.GoldStar, RankOf(q, 0));
        Assert.Equal(CoreQualityRank.SilverStar, RankOf(q, 1));
    }

    [Fact]
    public void PartialReadings_AreRankedAmongThemselves()
    {
        // Cores 1 and 3 failed to report; they must not be treated as the worst cores.
        var q = CoreQualityService.GetCoreQualities(4, [180, 0, 240, 0]);

        Assert.Equal(CoreQualityRank.SilverStar, RankOf(q, 0));
        Assert.Equal(CoreQualityRank.GoldStar, RankOf(q, 2));
        Assert.Equal(CoreQualityRank.Standard, RankOf(q, 1));
        Assert.Equal(0, q[1].CppcPerformance);
    }

    [Fact]
    public void ASingleReading_IsNotEnoughToRank()
    {
        var q = CoreQualityService.GetCoreQualities(4, [0, 0, 240, 0]);

        Assert.All(q.Values, v => Assert.Equal(CoreQualityRank.Standard, v.Rank));
    }

    [Fact]
    public void ShorterReadingListThanCoreCount_DoesNotThrow()
    {
        var q = CoreQualityService.GetCoreQualities(8, [200, 300]);

        Assert.Equal(8, q.Count);
        Assert.Equal(CoreQualityRank.GoldStar, RankOf(q, 1));
        Assert.Equal(CoreQualityRank.SilverStar, RankOf(q, 0));
    }
}
