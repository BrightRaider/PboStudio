using System.Text.Json;
using PboStudio.Core;

namespace PboStudio.Tests.Vectors;

internal static class InterruptedRunAdapters
{
    [Vector("interrupted-run/classify")]
    public static object? Classify(JsonElement i)
    {
        string dir = Path.Combine(Path.GetTempPath(), "pbo-vector-state-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(dir);
        try
        {
            DateTime written = DateTime.Now;
            if (i.Bool("hasState"))
                TestStateService.SaveState(dir, new ActiveRunState { CurrentCore = 4 });

            // SaveState stamps the time itself, so read it back and measure the boot against that.
            var saved = TestStateService.DetectInterruptedRun(dir);
            if (saved is not null) written = saved.LastUpdateTime;

            var run = TestStateService.ClassifyInterruptedRun(dir, written.AddSeconds(i.Int("bootOffsetSeconds")));
            return run is null ? null : new { systemRestarted = run.SystemRestarted };
        }
        finally
        {
            try { Directory.Delete(dir, true); } catch { }
        }
    }
}
