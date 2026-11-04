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

    // -- the Ryzen 9000 default, and the person's right to hold it back --------------------

    [Fact]
    public void ARyzen9000StartsAtItsOwnDefaultWhenNobodyChoseAnything()
    {
        Assert.Equal(-50, CurveOptimizerRange.Effective(-50, null, None));
    }

    /// <summary>
    /// The person can hold a Ryzen 9000 at the standard. This is why the choice is stored as "chosen"
    /// or "not chosen" rather than as a number: a stored -30 has to mean something deliberate.
    /// </summary>
    [Fact]
    public void ARyzen9000CanBeCappedAtTheStandard()
    {
        Assert.Equal(-30, CurveOptimizerRange.Effective(-50, -30, None));
        Assert.Equal(-40, CurveOptimizerRange.Effective(-50, -40, None));
    }

    /// <summary>
    /// Asking for the depth a part already defaults to changes nothing; that is what lets the box
    /// always show a number and the effective range still follow the processor.
    /// </summary>
    [Fact]
    public void ChoosingTheDefaultItselfIsAnEmptyChoice()
    {
        Assert.Equal(-50, CurveOptimizerRange.Effective(-50, -50, None));
        Assert.Equal(-30, CurveOptimizerRange.Effective(-30, -30, None));
    }

    /// <summary>
    /// A board that takes -30 and nothing deeper. The first deep write is refused, the processor
    /// reports -30, and the range comes back to it - whatever the model default or the box said.
    /// </summary>
    [Fact]
    public void ADeeperDefaultIsPulledBackToWhatTheBoardActuallyAccepts()
    {
        Assert.Equal(-30, CurveOptimizerRange.Effective(-50, null, None, appliedFloor: -30));
        Assert.Equal(-40, CurveOptimizerRange.Effective(-50, -50, None, appliedFloor: -40));
    }

    /// <summary>
    /// Hardware evidence never shrinks the range: a core held at -45 by the BIOS only ever widens
    /// it, and a person's cap is not overridden by a shallower reading.
    /// </summary>
    [Fact]
    public void WhatTheProcessorHoldsOnlyEverWidensTheRange()
    {
        Assert.Equal(-50, CurveOptimizerRange.Effective(-50, null, [-10, -20]));
        Assert.Equal(-45, CurveOptimizerRange.Effective(-30, -30, [-45]));
    }

    // -- what feeds it -----------------------------------------------

    /// <summary>The model's own default is a starting point: -50 for Zen 5, the standard for the rest.</summary>
    [Theory]
    [InlineData("AMD Ryzen 9 7950X3D 16-Core Processor", -30)]
    [InlineData("AMD Ryzen 9 9950X 16-Core Processor", -50)]
    [InlineData("AMD Ryzen 7 5800X3D 8-Core Processor", -30)]
    public void TheDefaultForEveryModelIsItsGenerationsOwn(string cpu, int expected)
    {
        Assert.Equal(expected, AutoTunerService.GetMaxNegativeMargin(cpu));
    }

    // -- the stored choice -------------------------------------------------------------

    [Fact]
    public void AnUntouchedSettingsFileHasNoChoice()
    {
        Assert.Null(new AppSettings().ChosenCurveLimit());
    }

    /// <summary>
    /// 1.1.1 wrote -30 into every settings file whether or not the box was touched. Reading that as
    /// a choice would pin every Ryzen 9000 that ever ran 1.1.1 to the standard for good.
    /// </summary>
    [Fact]
    public void ADefaultStoredBy111IsNotMistakenForAChoice()
    {
        Assert.Null(new AppSettings { CurveOptimizerLimit = -30 }.ChosenCurveLimit());
    }

    [Fact]
    public void ADeeperValueStoredBy111WasAChoiceAndIsKept()
    {
        Assert.Equal(-40, new AppSettings { CurveOptimizerLimit = -40 }.ChosenCurveLimit());
    }

    [Fact]
    public void TheNewFieldWinsOverTheOldOne()
    {
        var settings = new AppSettings { CurveOptimizerLimit = -40, CurveOptimizerLimitChosen = -30 };

        Assert.Equal(-30, settings.ChosenCurveLimit());   // a deliberate cap, kept
    }
}
