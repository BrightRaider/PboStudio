using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

public class TestStateServiceTests : IDisposable
{
    private readonly string _testDir;

    public TestStateServiceTests()
    {
        _testDir = Path.Combine(Path.GetTempPath(), $"pbo_test_state_{Guid.NewGuid():N}");
        Directory.CreateDirectory(_testDir);
    }

    public void Dispose()
    {
        if (Directory.Exists(_testDir))
        {
            try { Directory.Delete(_testDir, recursive: true); } catch { }
        }
    }

    [Fact]
    public void SaveState_And_LoadState_Roundtrip_WorksAtomically()
    {
        var state = new ActiveRunState
        {
            IsActive = true,
            CurrentCore = 4,
            CurrentIteration = 2,
            EngineName = "Prime95",
            SelectedCores = [0, 2, 4, 6],
            CompletedCores = [0, 2],
            CoreMargins = new Dictionary<int, int> { [0] = -25, [2] = -22, [4] = -18 },
        };

        TestStateService.SaveState(_testDir, state);

        var loaded = TestStateService.LoadState(_testDir);
        Assert.NotNull(loaded);
        Assert.True(loaded.IsActive);
        Assert.Equal(4, loaded.CurrentCore);
        Assert.Equal(2, loaded.CurrentIteration);
        Assert.Equal("Prime95", loaded.EngineName);
        Assert.Equal(4, loaded.SelectedCores.Count);
        Assert.Equal(-18, loaded.CoreMargins[4]);
    }

    [Fact]
    public void DetectInterruptedRun_ReturnsStateOnlyWhenActive()
    {
        var activeState = new ActiveRunState { IsActive = true, CurrentCore = 1 };
        TestStateService.SaveState(_testDir, activeState);

        var detected = TestStateService.DetectInterruptedRun(_testDir);
        Assert.NotNull(detected);

        var inactiveState = activeState with { IsActive = false };
        TestStateService.SaveState(_testDir, inactiveState);

        var notDetected = TestStateService.DetectInterruptedRun(_testDir);
        Assert.Null(notDetected);
    }

    [Fact]
    public void ClearState_RemovesFileAndTempFiles()
    {
        var state = new ActiveRunState { IsActive = true };
        TestStateService.SaveState(_testDir, state);

        string tempDummy = Path.Combine(_testDir, "state.json.tmp.dummy");
        File.WriteAllText(tempDummy, "temp");

        TestStateService.ClearState(_testDir);

        Assert.False(File.Exists(TestStateService.GetStateFilePath(_testDir)));
        Assert.False(File.Exists(tempDummy));
    }
}
