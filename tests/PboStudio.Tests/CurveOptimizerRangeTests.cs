using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

/// <summary>
/// 1.1.0 stated that Curve Optimizer is -30 to +30 on every AMD desktop part. It was not
/// sourced, and a Ryzen 9 7950X3D owner reported -40 working in the BIOS. These pin down how
/// the range is worked out now, including the two ways it must never be wrong: displaying a
/// value the processor holds as something it does not, and promising a depth that does not stick.
/// </summary>
public class CurveOptimizerRangeTests
{
    private static readonly int[] None = [];

    [Fact]
    public void WithNothingElseToGoOnItIsTheStandardRange()
    {
        Assert.Equal(-30, CurveOptimizerRange.Effective(-30, null, None));
    }

    // -- the person's own say ----------------------------------------

    [Fact]
    public void AskingForADeeperRangeGetsIt()
    {
        Assert.Equal(-40, CurveOptimizerRange.Effective(-30, -40, None));
    }

    [Fact]
    public void AskingForAShallowerRangeThanTheStandardIsIgnored()
    {
        Assert.Equal(-30, CurveOptimizerRange.Effective(-30, -10, None));
    }

    /// <summary>A bound on typing: nothing deeper than -50 has ever been reported.</summary>
    [Fact]
    public void NothingGoesPastTheWidestAnyoneHasClaimed()
    {
        Assert.Equal(-50, CurveOptimizerRange.Effective(-30, -80, None));
    }

    // -- what the processor itself shows -----------------------------

    /// <summary>
    /// The reporter's case. The BIOS set a core to -40; the program read it, showed -30, called
    /// the row edited, and "Apply live" would have written -30 over it. A value the processor
    /// holds is a value it accepts, and no setting outweighs that.
    /// </summary>
    [Fact]
    public void AValueTheProcessorReportsHoldingWidensTheRangeToIncludeIt()
    {
        Assert.Equal(-40, CurveOptimizerRange.Effective(-30, null, [-12, -40, -25]));
    }

    [Fact]
    public void ValuesInsideTheRangeChangeNothing()
    {
        Assert.Equal(-30, CurveOptimizerRange.Effective(-30, null, [-5, -20, -30]));
    }

    [Fact]
    public void WhateverTheProcessorReportsStillStopsAtTheWidest()
    {
        Assert.Equal(-50, CurveOptimizerRange.Effective(-30, null, [-200]));
    }

    // -- a write that did not stick ----------------------------------

    /// <summary>
    /// Asking for -45 and finding -40 applied means -40 is where the limit is, whatever was
    /// asked. Without this, a search would keep requesting values the processor quietly rounds.
    /// </summary>
    [Fact]
    public void AWriteThatWasClampedPullsTheRangeBack()
    {
        Assert.Equal(-40, CurveOptimizerRange.Effective(-30, -45, None, appliedFloor: -40));
    }

    [Fact]
    public void AClampNeverNarrowsBelowTheStandardRange()
    {
        // A "clamp" at -10 would be nonsense in the verified range; it is not believed.
        Assert.Equal(-40, CurveOptimizerRange.Effective(-30, -40, None, appliedFloor: -10));
    }

    // -- verifying a write -------------------------------------------

    [Theory]
    [InlineData(-45, -45, true)]    // kept
    [InlineData(-45, -40, false)]   // clamped
    [InlineData(-45, -30, false)]   // ignored entirely
    [InlineData(-45, null, true)]   // could not read it back: no evidence either way
    public void ADeepWriteIsKeptOnlyIfItReadsBackAsWritten(int requested, int? readBack, bool kept)
    {
        Assert.Equal(kept, CurveOptimizerRange.WasKept(requested, readBack));
    }

    /// <summary>
    /// Inside the standard range the write path is verified on hardware. A mismatch there is
    /// more likely a read that raced the write, and treating it as a refusal would break a
    /// path that works.
    /// </summary>
    [Theory]
    [InlineData(-30, -29)]
    [InlineData(-10, 0)]
    public void WritesInsideTheStandardRangeAreNeverSecondGuessed(int requested, int readBack)
    {
        Assert.True(CurveOptimizerRange.WasKept(requested, readBack));
    }

    // -- what feeds it -----------------------------------------------

    /// <summary>The model limit stays what it was: the safe default, not a measurement of every board.</summary>
    [Theory]
    [InlineData("AMD Ryzen 9 7950X3D 16-Core Processor")]
    [InlineData("AMD Ryzen 9 9950X 16-Core Processor")]
    [InlineData("AMD Ryzen 7 5800X3D 8-Core Processor")]
    public void TheDefaultForEveryModelIsStillTheStandardRange(string cpu)
    {
        Assert.Equal(-30, AutoTunerService.GetMaxNegativeMargin(cpu));
    }
}
