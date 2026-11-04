using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

/// <summary>
/// Windows lists the enabled cores; the SMU is addressed by physical slot, eight per chiplet.
/// The old mapping assumed "core N is slot N modulo the cores per chiplet" with a divisor that
/// was really eight slots. It is right for an 8-core and a 16-core part and wrong for every
/// 6+6 and 6-core one - so on a 7900X3D the whole second chiplet was written to the wrong place,
/// and a core could pass a test at a value it never held.
/// </summary>
public class SmuCoreMapTests
{
    private static SmuCoreSlot S(int ccd, int slot) => new(ccd, slot);

    // -- the parts where the old mapping happened to be right: it must not change ---------

    /// <summary>The Ryzen 7 5800X3D this program was verified on.</summary>
    [Fact]
    public void AnEightCorePartMapsStraightThrough()
    {
        var map = SmuCoreMap.Build([0u], 8)!;

        Assert.Equal(Enumerable.Range(0, 8).Select(i => S(0, i)), map);
    }

    [Fact]
    public void ASixteenCoreTwoChipletPartMapsStraightThrough()
    {
        var map = SmuCoreMap.Build([0u, 0u], 16)!;

        Assert.Equal(S(0, 7), map[7]);
        Assert.Equal(S(1, 0), map[8]);
        Assert.Equal(S(1, 7), map[15]);
    }

    // -- the case that was wrong ---------------------------------------------------------

    /// <summary>
    /// A 7900X3D, 5900X or 7900X: six cores on each of two chiplets, the top two slots fused off.
    /// The old divisor of eight sent Windows core 6 - the first core of the second chiplet - to
    /// chiplet 0, slot 6.
    /// </summary>
    [Fact]
    public void ATwelveCorePartPutsTheSecondChipletOnTheSecondChiplet()
    {
        uint topTwoOff = 0b1100_0000;
        var map = SmuCoreMap.Build([topTwoOff, topTwoOff], 12)!;

        Assert.Equal(S(0, 0), map[0]);
        Assert.Equal(S(0, 5), map[5]);
        Assert.Equal(S(1, 0), map[6]);   // the old code said (0, 6)
        Assert.Equal(S(1, 5), map[11]);
    }

    /// <summary>
    /// Fused-off slots are not always the top ones. "Slot N" can be a disabled core; the k-th core
    /// Windows lists is the k-th <em>enabled</em> slot.
    /// </summary>
    [Fact]
    public void ScatteredDisabledSlotsAreSkipped()
    {
        uint slots2And5Off = 0b0010_0100;
        var map = SmuCoreMap.Build([slots2And5Off], 6)!;

        Assert.Equal([S(0, 0), S(0, 1), S(0, 3), S(0, 4), S(0, 6), S(0, 7)], map);
    }

    [Fact]
    public void ASixCoreSingleChipletPartWithTheTopSlotsOffIsStillStraight()
    {
        var map = SmuCoreMap.Build([0b1100_0000u], 6)!;

        Assert.Equal(Enumerable.Range(0, 6).Select(i => S(0, i)), map);
    }

    [Fact]
    public void ChipletsCanHaveDifferentNumbersOfCores()
    {
        // 8 + 6: an unusual binning, but the arithmetic must not assume symmetry.
        var map = SmuCoreMap.Build([0u, 0b1100_0000u], 14)!;

        Assert.Equal(S(0, 7), map[7]);
        Assert.Equal(S(1, 0), map[8]);
        Assert.Equal(S(1, 5), map[13]);
    }

    // -- when the numbers do not add up --------------------------------------------------

    /// <summary>
    /// Fuses and Windows disagreeing about how many cores there are means one of them is wrong,
    /// and a mapping built on it would be a guess dressed as a fact.
    /// </summary>
    [Fact]
    public void AMapThatDoesNotMatchWindowsIsRefused()
    {
        Assert.Null(SmuCoreMap.Build([0u], 6));            // fuses say 8, Windows says 6
        Assert.Null(SmuCoreMap.Build([0b11u], 8));         // fuses say 6, Windows says 8
    }

    [Fact]
    public void NoFuseDataMeansNoMap()
    {
        Assert.Null(SmuCoreMap.Build(null, 8));
        Assert.Null(SmuCoreMap.Build([], 8));
        Assert.Null(SmuCoreMap.Build([0u], 0));
    }

    // -- the fallback --------------------------------------------------------------------

    [Fact]
    public void TheFallbackDividesTheCoresWindowsReportsEvenlyBetweenChiplets()
    {
        // The twelve-core case, without a fuse map: still right, where the old code was not.
        Assert.Equal(S(1, 0), SmuCoreMap.Fallback(6, ccdCount: 2, windowsCoreCount: 12));
        Assert.Equal(S(1, 5), SmuCoreMap.Fallback(11, ccdCount: 2, windowsCoreCount: 12));
        Assert.Equal(S(0, 5), SmuCoreMap.Fallback(5, ccdCount: 2, windowsCoreCount: 12));
    }

    [Fact]
    public void TheFallbackAgreesWithTheOldMappingOnTheVerifiedPart()
    {
        for (int core = 0; core < 8; core++)
            Assert.Equal(S(0, core), SmuCoreMap.Fallback(core, ccdCount: 1, windowsCoreCount: 8));
    }

    [Theory]
    [InlineData(-1, 2, 12)]
    [InlineData(12, 2, 12)]
    [InlineData(0, 0, 12)]
    [InlineData(0, 2, 0)]
    [InlineData(0, 5, 12)]   // twelve cores do not divide over five chiplets
    public void TheFallbackRefusesWhatItCannotPlace(int core, int ccds, int cores)
    {
        Assert.Null(SmuCoreMap.Fallback(core, ccds, cores));
    }

    /// <summary>The map and the fallback agree whenever the fuse map shows nothing unusual.</summary>
    [Theory]
    [InlineData(8, 1, 0u)]
    [InlineData(16, 2, 0u)]
    [InlineData(12, 2, 0b1100_0000u)]
    [InlineData(6, 1, 0b1100_0000u)]
    public void MapAndFallbackAgreeWhenTheTopSlotsAreTheOnesFusedOff(int cores, int ccds, uint mask)
    {
        var map = SmuCoreMap.Build(Enumerable.Repeat(mask, ccds).ToList(), cores)!;

        for (int core = 0; core < cores; core++)
            Assert.Equal(map[core], SmuCoreMap.Fallback(core, ccds, cores));
    }
}
