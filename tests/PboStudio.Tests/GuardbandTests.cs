using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

/// <summary>
/// The headroom between the value a core is measured to survive and the value it is locked at.
///
/// This exists because of a real failure. A Ryzen 7 5800X3D passed hours of Prime95 and
/// y-cruncher, then threw two fatal WHEA 18 cache-hierarchy errors during an ordinary game —
/// APIC 0 and APIC 9, i.e. physical cores 0 and 4 — and hard-reset. A stress test finds the
/// boundary margin under that particular load; a light-load game drives the core to its highest
/// boost clock, where a negative offset has the least voltage left to give. Locking the
/// boundary hands the user a value with nothing in reserve for the case the test never produced.
/// </summary>
public class GuardbandTests
{
    [Fact]
    public void HeadroomMovesTheLockedValueAwayFromTheBoundary()
    {
        Assert.Equal(-27, AutoTunerService.ApplyGuardband(-30, 3));
        Assert.Equal(-25, AutoTunerService.ApplyGuardband(-30, 5));
    }

    [Fact]
    public void ZeroHeadroomLocksTheMeasuredBoundaryItself()
    {
        Assert.Equal(-30, AutoTunerService.ApplyGuardband(-30, 0));
    }

    [Fact]
    public void HeadroomNeverPushesIntoAPositiveOffset()
    {
        // A core that only survives at -1 must not be handed +2.
        Assert.Equal(0, AutoTunerService.ApplyGuardband(-1, 3));
        Assert.Equal(0, AutoTunerService.ApplyGuardband(0, 5));
    }

    [Fact]
    public void ANegativeGuardbandIsIgnoredRatherThanMakingThingsWorse()
    {
        Assert.Equal(-30, AutoTunerService.ApplyGuardband(-30, -5));
    }

    // ── the three places a core gets locked ──────────────────────

    [Fact]
    public void LockingAtTheChipFloorKeepsItsHeadroom()
    {
        var step = AutoTunerService.CalculateNextStep(
            coreIndex: 0, currentMargin: -30, passed: true,
            AutoTunerMode.Fein, maxLimit: -30, isGerman: false, state: null, guardband: 3);

        Assert.True(step.CoreLocked);
        Assert.Equal(-30, step.OldMargin);
        Assert.Equal(-27, step.NextMargin);
    }

    [Fact]
    public void LockingAfterClimbingBackKeepsItsHeadroom()
    {
        var ascending = new AutoTunerCoreState(Ascending: true);

        var step = AutoTunerService.CalculateNextStep(
            coreIndex: 0, currentMargin: -24, passed: true,
            AutoTunerMode.Fein, maxLimit: -30, isGerman: false, state: ascending, guardband: 3);

        Assert.True(step.CoreLocked);
        Assert.Equal(-21, step.NextMargin);
    }

    [Fact]
    public void SettlingBackOntoAProvenValueKeepsItsHeadroom()
    {
        var known = new AutoTunerCoreState(LastKnownGood: -26);

        var step = AutoTunerService.CalculateNextStep(
            coreIndex: 0, currentMargin: -29, passed: false,
            AutoTunerMode.Fein, maxLimit: -30, isGerman: false, state: known, guardband: 4);

        Assert.True(step.CoreLocked);
        Assert.Equal(-22, step.NextMargin);
    }

    [Fact]
    public void TheAdviceNamesTheHeadroomSoTheLockedValueIsNotASurprise()
    {
        var step = AutoTunerService.CalculateNextStep(
            coreIndex: 0, currentMargin: -30, passed: true,
            AutoTunerMode.Fein, maxLimit: -30, isGerman: false, state: null, guardband: 3);

        Assert.Contains("-27", step.Advice);
        Assert.Contains("headroom", step.Advice);
        Assert.Contains("-30", step.Advice);
    }

    [Fact]
    public void TheSearchItselfIsUnaffectedUntilSomethingLocks()
    {
        // Still descending: the guardband applies at the lock, not to every step.
        var step = AutoTunerService.CalculateNextStep(
            coreIndex: 0, currentMargin: -20, passed: true,
            AutoTunerMode.Grob, maxLimit: -30, isGerman: false, state: null, guardband: 3);

        Assert.False(step.CoreLocked);
        Assert.Equal(-23, step.NextMargin);
    }

    /// <summary>The measured boundary stays on record even though it is not what gets applied.</summary>
    [Fact]
    public void WhatWasActuallyMeasuredIsStillRemembered()
    {
        var step = AutoTunerService.CalculateNextStep(
            coreIndex: 0, currentMargin: -30, passed: true,
            AutoTunerMode.Fein, maxLimit: -30, isGerman: false, state: null, guardband: 3);

        Assert.Equal(-30, step.State.LastKnownGood);
    }

    // ── preferred cores ──────────────────────────────────────────

    [Fact]
    public void PreferredCoresGetMoreHeadroomThanTheRest()
    {
        var plan = new TestPlan { Guardband = 3, PreferredCores = new HashSet<int> { 0, 4 } };

        Assert.Equal(3 + AutoTunerService.PreferredCoreExtraGuardband, plan.GuardbandFor(0));
        Assert.Equal(3 + AutoTunerService.PreferredCoreExtraGuardband, plan.GuardbandFor(4));
        Assert.Equal(3, plan.GuardbandFor(2));
    }

    [Fact]
    public void WithoutAnyRankingEveryCoreGetsTheSameHeadroom()
    {
        var plan = new TestPlan { Guardband = 4 };
        Assert.Equal(4, plan.GuardbandFor(0));
    }

    // ── micro-bursts ─────────────────────────────────────────────

    [Fact]
    public void PeriodicPausingExpectsTheCoreToBeBusyThroughout()
    {
        Assert.Equal(1.0, new TestPlan().ExpectedDutyCycle);
    }

    /// <summary>
    /// Micro-bursting idles the core on purpose, so the occupancy check has to expect that —
    /// otherwise every micro-burst slot would be reported as a core that went idle.
    /// </summary>
    [Fact]
    public void MicroBurstingExpectsOnlyItsDutyCycle()
    {
        var plan = new TestPlan
        {
            Transient = TransientMode.MicroBurst,
            BurstLoad = TimeSpan.FromMilliseconds(300),
            BurstIdle = TimeSpan.FromMilliseconds(100),
        };

        Assert.Equal(0.75, plan.ExpectedDutyCycle, precision: 3);
    }

    [Fact]
    public void AHarsherDutyCycleLowersTheExpectationFurther()
    {
        var plan = new TestPlan
        {
            Transient = TransientMode.MicroBurst,
            BurstLoad = TimeSpan.FromMilliseconds(100),
            BurstIdle = TimeSpan.FromMilliseconds(300),
        };

        Assert.Equal(0.25, plan.ExpectedDutyCycle, precision: 3);
    }

    [Fact]
    public void ADegenerateBurstSettingFallsBackToFullOccupancy()
    {
        var plan = new TestPlan
        {
            Transient = TransientMode.MicroBurst,
            BurstLoad = TimeSpan.Zero,
            BurstIdle = TimeSpan.Zero,
        };

        Assert.Equal(1.0, plan.ExpectedDutyCycle);
    }

    /// <summary>
    /// The micro-burst profile runs the coolest load in the set, not the hottest. The failure it
    /// reproduces happens at light load, where the core boosts highest and a negative offset has
    /// the least voltage to give; AVX2 would heat the core and pull the clock away from it.
    /// </summary>
    [Fact]
    public void TheMicroBurstProfileUsesTheColdestLoadOnOneThread()
    {
        var profile = TestProfiles.For("AMD Ryzen 7 5800X3D", isGerman: false)
            .Single(p => p.Id == ProfileId.MicroBurst);

        Assert.Equal(TransientMode.MicroBurst, profile.Transient);
        Assert.Equal(Prime95Mode.Sse, profile.Mode);
        Assert.Equal(1, profile.Threads);
    }
}
