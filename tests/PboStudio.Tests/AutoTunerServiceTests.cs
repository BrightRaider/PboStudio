using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

public class AutoTunerServiceTests
{
    private static AutoTunerStepResult Step(
        int margin, bool passed, AutoTunerMode mode = AutoTunerMode.Grob,
        AutoTunerCoreState? state = null, int maxLimit = AutoTunerService.DefaultBiosLimit) =>
        AutoTunerService.CalculateNextStep(0, margin, passed, mode, maxLimit, state: state);

    // ─────────────────────────── descending ───────────────────────────

    [Fact]
    public void Passed_GrobMode_StepsDownByThree()
    {
        var result = Step(-15, passed: true);

        Assert.Equal(-15, result.OldMargin);
        Assert.Equal(-18, result.NextMargin);
        Assert.False(result.CoreLocked);
        Assert.Equal(-15, result.State.LastKnownGood);
    }

    [Fact]
    public void Passed_FeinMode_StepsDownByOne()
    {
        var result = Step(-20, passed: true, AutoTunerMode.Fein);

        Assert.Equal(-21, result.NextMargin);
        Assert.False(result.CoreLocked);
    }

    [Fact]
    public void Passed_ClampsAtBiosLimit()
    {
        Assert.Equal(-30, Step(-29, passed: true).NextMargin);

        var atLimit = Step(-30, passed: true);
        Assert.Equal(-30, atLimit.NextMargin);
        Assert.True(atLimit.CoreLocked);
    }

    // ─────────────────────────── ascending after a failure ───────────────────────────

    [Fact]
    public void FailedWithoutAnyPass_KeepsSearchingUpwardInsteadOfLocking()
    {
        // The whole point: a value that has never held must not be handed back as "done".
        var result = Step(-30, passed: false);

        Assert.Equal(-27, result.NextMargin);
        Assert.False(result.CoreLocked);
        Assert.True(result.State.Ascending);
    }

    [Fact]
    public void FailedWithoutAnyPass_FeinMode_StepsUpByOne()
    {
        var result = Step(-28, passed: false, AutoTunerMode.Fein);

        Assert.Equal(-27, result.NextMargin);
        Assert.False(result.CoreLocked);
    }

    [Fact]
    public void FirstPassWhileAscending_LocksThere()
    {
        var ascending = new AutoTunerCoreState(LastKnownGood: null, Ascending: true);
        var result = Step(-24, passed: true, state: ascending);

        Assert.Equal(-24, result.NextMargin);
        Assert.True(result.CoreLocked);
    }

    [Fact]
    public void StartingAtTheLimit_WalksUpUntilItHolds()
    {
        // Start -30, fail, fail, then pass: the full journey the user asked about.
        var state = new AutoTunerCoreState();
        int margin = -30;

        var first = Step(margin, passed: false, state: state);
        margin = first.NextMargin; state = first.State;
        Assert.Equal(-27, margin);
        Assert.False(first.CoreLocked);

        var second = Step(margin, passed: false, state: state);
        margin = second.NextMargin; state = second.State;
        Assert.Equal(-24, margin);
        Assert.False(second.CoreLocked);

        var third = Step(margin, passed: true, state: state);
        Assert.Equal(-24, third.NextMargin);
        Assert.True(third.CoreLocked);
    }

    // ─────────────────────────── failure after a known-good value ───────────────────────────

    [Fact]
    public void FailedAfterAKnownGoodValue_ReturnsToItWithoutRetesting()
    {
        // -18 already survived a full slot, so there is nothing to prove by testing it again.
        var state = new AutoTunerCoreState(LastKnownGood: -18);
        var result = Step(-21, passed: false, state: state);

        Assert.Equal(-18, result.NextMargin);
        Assert.True(result.CoreLocked);
    }

    [Fact]
    public void DescendThenFail_SettlesOnTheLastPassingValue()
    {
        var state = new AutoTunerCoreState();

        var a = Step(-10, passed: true, state: state);       // -10 holds -> try -13
        state = a.State;
        Assert.Equal(-13, a.NextMargin);

        var b = Step(-13, passed: true, state: state);       // -13 holds -> try -16
        state = b.State;
        Assert.Equal(-16, b.NextMargin);

        var c = Step(-16, passed: false, state: state);      // -16 fails -> back to -13
        Assert.Equal(-13, c.NextMargin);
        Assert.True(c.CoreLocked);
    }

    // ─────────────────────────── the pathological case ───────────────────────────

    [Fact]
    public void FailingWithoutUndervolt_StopsAndSaysTheCurveIsNotTheCause()
    {
        var result = Step(0, passed: false);

        Assert.Equal(0, result.NextMargin);
        Assert.True(result.CoreLocked);
        Assert.Contains("not the Curve Optimizer", result.Advice, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void AscendingNeverGoesPositive()
    {
        var result = Step(-1, passed: false);

        Assert.Equal(0, result.NextMargin);
        Assert.False(result.CoreLocked);
    }

    // ─────────────────────────── runtime estimate ───────────────────────────

    [Theory]
    [InlineData(-30, 3, 3)]     // at the limit, capped by the pass budget
    [InlineData(-30, 20, 11)]   // worst case is the full climb back to 0: 10 steps plus the first slot
    [InlineData(0, 20, 11)]     // mirror image: the full descent to -30
    [InlineData(-15, 20, 6)]    // halfway down, either leg is five steps
    [InlineData(-15, 2, 2)]     // never promises more slots than passes allow
    public void WorstCaseSlots_IsBoundedByThePassBudget(int margin, int maxPasses, int expected)
    {
        int slots = AutoTunerService.WorstCaseSlots(margin, -30, AutoTunerMode.Grob, maxPasses);
        Assert.Equal(expected, slots);
    }

    /// <summary>
    /// This asserted -50 for Zen 5, on the reading that the generation has an extended range.
    /// It does not. Curve Optimizer is -30 to +30 on every AMD desktop part that has it; what
    /// Zen 5 added is Curve Shaper, a second and separate set of offsets across temperature and
    /// frequency bands, which this program does not write. A search allowed to run to -50 ends
    /// by reporting values that cannot be entered in a BIOS, which is the whole deliverable.
    /// </summary>
    [Theory]
    [InlineData("AMD Ryzen 9 9950X 16-Core Processor")]
    [InlineData("AMD Ryzen 7 9800X3D 8-Core Processor")]
    [InlineData("AMD Ryzen 7 7800X3D 8-Core Processor")]
    [InlineData("AMD Ryzen 7 5800X3D 8-Core Processor")]
    public void GetMaxNegativeMargin_IsMinusThirtyOnEveryPartThatHasTheFeature(string cpu)
    {
        Assert.Equal(-30, AutoTunerService.GetMaxNegativeMargin(cpu));
    }
}
