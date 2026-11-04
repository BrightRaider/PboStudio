using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

/// <summary>
/// Any run that ends abnormally leaves its state file: a crash, but also a kill from the task
/// manager, a crash of this program, or a window closed mid-run. Every one was reported at the next
/// start as "unexpected system restart", and the margin the core held was written into the tuner's
/// memory as a failure for good. A core that never failed was capped for something it did not do.
/// </summary>
public sealed class InterruptedRunTests : IDisposable
{
    private readonly string _dir = Path.Combine(Path.GetTempPath(), "pbo-state-" + Guid.NewGuid().ToString("N"));

    public InterruptedRunTests() => Directory.CreateDirectory(_dir);
    public void Dispose() { try { Directory.Delete(_dir, true); } catch { } }

    private void WriteState(int core = 4)
    {
        TestStateService.SaveState(_dir, new ActiveRunState
        {
            CurrentCore = core,
            CoreMargins = new Dictionary<int, int> { [core] = -28 },
        });
    }

    [Fact]
    public void NoStateFileMeansNothingWasInterrupted()
    {
        Assert.Null(TestStateService.ClassifyInterruptedRun(_dir, DateTime.Now));
    }

    /// <summary>The machine started again after the run last wrote its state: that is a crash.</summary>
    [Fact]
    public void AComputerThatRestartedAfterTheRunIsACrash()
    {
        WriteState();
        var bootedAfter = DateTime.Now.AddMinutes(5);

        var run = TestStateService.ClassifyInterruptedRun(_dir, bootedAfter);

        Assert.NotNull(run);
        Assert.True(run!.SystemRestarted);
        Assert.Equal(4, run.State.CurrentCore);
    }

    /// <summary>
    /// The computer has been up since before the run wrote its state, so it never went down. The
    /// program was closed or killed, and nothing about the core is known to have failed.
    /// </summary>
    [Fact]
    public void AComputerThatNeverRestartedMeansTheProgramWasClosedNotThatTheCoreFailed()
    {
        WriteState();
        var bootedLongBefore = DateTime.Now.AddHours(-12);

        var run = TestStateService.ClassifyInterruptedRun(_dir, bootedLongBefore);

        Assert.NotNull(run);
        Assert.False(run!.SystemRestarted);
    }

    [Fact]
    public void TheStartTimeComesFromTheUptimeAndIsInThePast()
    {
        var started = TestStateService.SystemStartTime();

        Assert.True(started < DateTime.Now);
        Assert.True(started > DateTime.Now.AddYears(-1));
    }
}
