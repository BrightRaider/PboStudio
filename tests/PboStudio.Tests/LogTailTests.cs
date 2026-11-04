using System.Diagnostics;
using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

/// <summary>
/// Both engines report a failed calculation by writing a line that this program polls once a
/// second. The poll read to the end of the file and resumed from the file's length, so a line
/// that was half written at that moment had its first half inspected and its second half never:
/// "FATAL ERROR" split across two polls matched neither, and the core passed. A missed failure
/// is the one mistake a stability test cannot afford.
/// </summary>
public sealed class LogTailTests : IDisposable
{
    private readonly string _dir = Path.Combine(Path.GetTempPath(), "pbo-logtail-" + Guid.NewGuid().ToString("N"));
    private string File1 => Path.Combine(_dir, "results.txt");

    public LogTailTests() => Directory.CreateDirectory(_dir);

    public void Dispose()
    {
        try { Directory.Delete(_dir, recursive: true); } catch { }
    }

    private void Append(string text) => File.AppendAllText(File1, text);

    [Fact]
    public void WholeLinesAreReturnedOnce()
    {
        Append("one\ntwo\n");
        long offset = 0;

        Assert.Equal(["one", "two"], LogTail.ReadNewLines(File1, ref offset));
        Assert.Empty(LogTail.ReadNewLines(File1, ref offset));
    }

    /// <summary>The bug: the second half of a line was never inspected.</summary>
    [Fact]
    public void ALineStillBeingWrittenIsHeldBackUntilItIsFinished()
    {
        Append("fine\nFATAL ER");
        long offset = 0;

        Assert.Equal(["fine"], LogTail.ReadNewLines(File1, ref offset));

        Append("ROR: Rounding was 0.5\n");

        Assert.Equal(["FATAL ERROR: Rounding was 0.5"], LogTail.ReadNewLines(File1, ref offset));
    }

    [Fact]
    public void TheOffsetDoesNotMoveOverAnUnfinishedLine()
    {
        Append("no newline yet");
        long offset = 0;

        Assert.Empty(LogTail.ReadNewLines(File1, ref offset));
        Assert.Equal(0, offset);
    }

    [Fact]
    public void WindowsLineEndingsAreStripped()
    {
        Append("a\r\nb\r\n");
        long offset = 0;

        Assert.Equal(["a", "b"], LogTail.ReadNewLines(File1, ref offset));
    }

    /// <summary>A file recreated or truncated under us is read again from the start.</summary>
    [Fact]
    public void AShrunkFileIsReadFromTheStartAgain()
    {
        Append("first run line\n");
        long offset = 0;
        LogTail.ReadNewLines(File1, ref offset);

        File.WriteAllText(File1, "new\n");

        Assert.Equal(["new"], LogTail.ReadNewLines(File1, ref offset));
    }

    [Fact]
    public void AMissingFileYieldsNothingRatherThanThrowing()
    {
        long offset = 0;

        Assert.Empty(LogTail.ReadNewLines(Path.Combine(_dir, "absent.txt"), ref offset));
    }

    [Fact]
    public void EveryLineOfAManyLineBatchComesBack()
    {
        var lines = Enumerable.Range(0, 500).Select(i => $"line {i}").ToList();
        Append(string.Join("\n", lines) + "\n");
        long offset = 0;

        Assert.Equal(lines, LogTail.ReadNewLines(File1, ref offset));
    }

    // -- end to end through the Prime95 session --------------------------------------

    /// <summary>
    /// The failure the whole class exists to prevent, seen from the outside: an error line that
    /// arrives in two pieces must still be reported, exactly once, when it completes.
    /// </summary>
    [Fact]
    public void ASplitPrimeErrorLineIsStillReported()
    {
        // A live process so the session does not also report the worker as gone.
        using var self = Process.GetCurrentProcess();
        var session = new Prime95Session(self, File1);

        Append("Self-test 8960K passed!\nFATAL ERROR: Rou");
        Assert.Empty(session.DrainFailures());

        Append("nding was 0.5, expected less than 0.4\n");
        var failures = session.DrainFailures();

        var failure = Assert.Single(failures);
        Assert.Equal(FailureKind.CalculationError, failure.Kind);
        Assert.Contains("FATAL ERROR", failure.Detail);

        // And not again on the next poll.
        Assert.Empty(session.DrainFailures());
    }

    /// <summary>
    /// The case that was actually missed. When the marker itself is cut by the poll boundary the
    /// old code saw "FATAL ER" and then "ROR: ...": neither half matched, the offset had already
    /// moved on, and the failure vanished.
    /// </summary>
    [Fact]
    public void AnErrorMarkerCutInTheMiddleIsStillReported()
    {
        using var self = Process.GetCurrentProcess();
        var session = new Prime95Session(self, File1);

        Append("Self-test 8960K passed!\nFATAL ER");
        Assert.Empty(session.DrainFailures());

        Append("ROR: something went wrong\n");

        var failure = Assert.Single(session.DrainFailures());
        Assert.Equal(FailureKind.CalculationError, failure.Kind);
    }

    [Fact]
    public void AnErrorLineThatArrivesWholeIsReported()
    {
        using var self = Process.GetCurrentProcess();
        var session = new Prime95Session(self, File1);

        Append("Hardware failure detected, consult stress.txt file.\n");

        Assert.Single(session.DrainFailures());
    }

    [Fact]
    public void APassedLineIsNotAFailure()
    {
        using var self = Process.GetCurrentProcess();
        var session = new Prime95Session(self, File1);

        Append("Self-test 1344K passed!\n");

        Assert.Empty(session.DrainFailures());
    }
}
