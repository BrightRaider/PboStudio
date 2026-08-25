using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

/// <summary>
/// The auto-tuner's long-term memory. Before it existed, the only record that a core had taken
/// the machine down at a given value was a line in a log file no code reads — so the next run
/// walked it straight back to that value.
/// </summary>
public class CoreKnowledgeServiceTests
{
    private const int ChipLimit = -30;

    [Fact]
    public void WithoutEvidenceTheFloorIsTheChipLimit()
    {
        Assert.Equal(ChipLimit, new CoreKnowledge().FloorFor(ChipLimit));
    }

    [Fact]
    public void AFailureRaisesTheFloorByOnePoint()
    {
        var k = new CoreKnowledge(WorstFailed: -30);
        Assert.Equal(-29, k.FloorFor(ChipLimit));
    }

    [Fact]
    public void EverythingAtOrBelowAFailedValueCountsAsKnownBad()
    {
        var k = new CoreKnowledge(WorstFailed: -28);

        Assert.True(k.IsKnownBad(-28));
        Assert.True(k.IsKnownBad(-30));
        Assert.False(k.IsKnownBad(-27));
    }

    [Fact]
    public void OnlyAMoreAggressivePassIsNews()
    {
        var knowledge = new Dictionary<int, CoreKnowledge>();

        CoreKnowledgeService.RecordPass(knowledge, 0, -20);
        CoreKnowledgeService.RecordPass(knowledge, 0, -25);
        Assert.Equal(-25, knowledge[0].BestPassed);

        // A milder success does not undo a stronger one.
        CoreKnowledgeService.RecordPass(knowledge, 0, -10);
        Assert.Equal(-25, knowledge[0].BestPassed);
    }

    [Fact]
    public void AMilderFailureWidensTheBadRegion()
    {
        var knowledge = new Dictionary<int, CoreKnowledge>();

        CoreKnowledgeService.RecordFailure(knowledge, 1, -30);
        Assert.Equal(-30, knowledge[1].WorstFailed);

        // Failing at -27 means everything from -27 down is now suspect.
        CoreKnowledgeService.RecordFailure(knowledge, 1, -27);
        Assert.Equal(-27, knowledge[1].WorstFailed);
        Assert.Equal(-26, knowledge[1].FloorFor(ChipLimit));

        // A harsher failure adds nothing that was not already implied.
        CoreKnowledgeService.RecordFailure(knowledge, 1, -30);
        Assert.Equal(-27, knowledge[1].WorstFailed);
    }

    [Fact]
    public void AValueThatLaterFailsStopsCountingAsThePass()
    {
        var knowledge = new Dictionary<int, CoreKnowledge>();

        CoreKnowledgeService.RecordPass(knowledge, 2, -28);
        CoreKnowledgeService.RecordFailure(knowledge, 2, -28);

        Assert.Null(knowledge[2].BestPassed);
        Assert.Equal(-28, knowledge[2].WorstFailed);
    }

    [Fact]
    public void TheFloorNeverGoesMilderThanTheChipAllows()
    {
        // A core that fails even at 0 must not produce a positive floor.
        var k = new CoreKnowledge(WorstFailed: 0);
        Assert.Equal(1, k.FloorFor(ChipLimit));
    }

    /// <summary>
    /// The case this was built for: core 1 of a 5800X3D crashed the machine at -30 on
    /// 2026-08-24 and passed at -25. The auto-tuner must search -26 to -29 and never offer -30.
    /// </summary>
    [Fact]
    public void TheCrashedCoreIsNeverWalkedBackIntoItsCrash()
    {
        var knowledge = new Dictionary<int, CoreKnowledge>();
        CoreKnowledgeService.RecordFailure(knowledge, 1, -30);
        CoreKnowledgeService.RecordPass(knowledge, 1, -25);

        var floors = CoreKnowledgeService.FloorsFor(knowledge, ChipLimit);
        var plan = new TestPlan { MaxNegativeMargin = ChipLimit, PerCoreFloor = floors };

        Assert.Equal(-29, plan.FloorFor(1));

        // Cores nothing is known about keep the chip limit.
        Assert.Equal(-30, plan.FloorFor(4));

        // And the descent stops there: at -29 the tuner locks instead of stepping to -30.
        var step = AutoTunerService.CalculateNextStep(
            coreIndex: 1, currentMargin: -29, passed: true,
            AutoTunerMode.Fein, plan.FloorFor(1), isGerman: false, null);

        Assert.True(step.CoreLocked);
        Assert.Equal(-29, step.NextMargin);
    }

    [Fact]
    public void RoundTripsThroughDiskWithTheBiosStamp()
    {
        using var dir = new TempDir();

        var knowledge = new Dictionary<int, CoreKnowledge>
        {
            [0] = new(BestPassed: -20),
            [1] = new(BestPassed: -25, WorstFailed: -30),
        };

        CoreKnowledgeService.Save(dir.Path, knowledge, "2.20");
        var loaded = CoreKnowledgeService.Load(dir.Path);

        Assert.Equal(2, loaded.Cores.Count);
        Assert.Equal(-25, loaded.Cores[1].BestPassed);
        Assert.Equal(-30, loaded.Cores[1].WorstFailed);
        Assert.Equal("2.20", loaded.BiosVersion);
    }

    /// <summary>A file written before the BIOS stamp existed must still load.</summary>
    [Fact]
    public void ReadsTheOlderBareCoreMap()
    {
        using var dir = new TempDir();

        File.WriteAllText(CoreKnowledgeService.PathFor(dir.Path),
            """{ "1": { "BestPassed": -25, "WorstFailed": -30 } }""");

        var loaded = CoreKnowledgeService.Load(dir.Path);

        Assert.Single(loaded.Cores);
        Assert.Equal(-30, loaded.Cores[1].WorstFailed);
        Assert.Equal("", loaded.BiosVersion);
    }

    [Fact]
    public void AMissingOrCorruptFileYieldsAnEmptyMemoryRatherThanThrowing()
    {
        using var dir = new TempDir();

        Assert.Empty(CoreKnowledgeService.Load(dir.Path).Cores);

        File.WriteAllText(CoreKnowledgeService.PathFor(dir.Path), "{ this is not json");
        Assert.Empty(CoreKnowledgeService.Load(dir.Path).Cores);
    }

    [Fact]
    public void ResetForgetsEverythingAndTheFloorGoesBackToTheChipLimit()
    {
        using var dir = new TempDir();

        var knowledge = new Dictionary<int, CoreKnowledge> { [1] = new(WorstFailed: -30) };
        CoreKnowledgeService.Save(dir.Path, knowledge, "2.20");

        CoreKnowledgeService.Reset(dir.Path);

        var loaded = CoreKnowledgeService.Load(dir.Path);
        Assert.Empty(loaded.Cores);

        var plan = new TestPlan
        {
            MaxNegativeMargin = ChipLimit,
            PerCoreFloor = CoreKnowledgeService.FloorsFor(loaded.Cores, ChipLimit),
        };
        Assert.Equal(ChipLimit, plan.FloorFor(1));
    }

    [Fact]
    public void ResettingOneCoreLeavesTheOthersAlone()
    {
        using var dir = new TempDir();

        var knowledge = new Dictionary<int, CoreKnowledge>
        {
            [0] = new(BestPassed: -20),
            [1] = new(WorstFailed: -30),
        };

        CoreKnowledgeService.Reset(dir.Path, knowledge, core: 1);

        var loaded = CoreKnowledgeService.Load(dir.Path);
        Assert.Single(loaded.Cores);
        Assert.Equal(-20, loaded.Cores[0].BestPassed);
    }

    [Fact]
    public void CoresWithKnownBadCountsOnlyTheOnesThatFailed()
    {
        var knowledge = new Dictionary<int, CoreKnowledge>
        {
            [0] = new(BestPassed: -30),
            [1] = new(BestPassed: -25, WorstFailed: -30),
            [2] = new(BestPassed: -30),
        };

        Assert.Equal(1, CoreKnowledgeService.CoresWithKnownBad(knowledge));
    }

    private sealed class TempDir : IDisposable
    {
        public string Path { get; } = System.IO.Path.Combine(
            System.IO.Path.GetTempPath(), "pbostudio-knowledge-" + Guid.NewGuid().ToString("N"));

        public TempDir() => Directory.CreateDirectory(Path);

        public void Dispose()
        {
            try { if (Directory.Exists(Path)) Directory.Delete(Path, recursive: true); } catch { }
        }
    }
}
