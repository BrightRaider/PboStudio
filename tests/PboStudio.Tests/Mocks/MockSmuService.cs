using PboStudio.Core;

namespace PboStudio.Tests.Mocks;

public sealed class MockSmuService : ISmuService
{
    private readonly Dictionary<int, int> _margins = new();

    public string? InitError { get; set; }
    public bool IsAvailable { get; set; } = true;
    public string CpuName { get; set; } = "AMD Ryzen 7 5800X3D 8-Core Processor";
    public int PhysicalCores { get; set; } = 8;
    public CurveOptimizerSupport Support { get; set; } = CurveOptimizerSupport.Full;
    public int? ReportedCcdCount { get; set; } = 1;

    public bool ReadsAllZero() => _margins.Values.All(m => m == 0);

    public int? ReadCurveOptimizer(int core) =>
        _margins.TryGetValue(core, out int val) ? val : 0;

    public bool WriteCurveOptimizer(int core, int margin)
    {
        _margins[core] = margin;
        return true;
    }

    public bool WriteCurveOptimizerAllCores(int margin)
    {
        for (int i = 0; i < PhysicalCores; i++) _margins[i] = margin;
        return true;
    }

    public PboSnapshot? ReadPbo() =>
        new(Scalar: 1.0f, BoostMhz: 4550, PptLimitW: 142, PptNowW: 75, TdcLimitA: 95, TdcNowA: 40, EdcLimitA: 140, EdcNowA: 60, TempLimitC: 90, TempNowC: 62.5f);

    public void Dispose() { }
}
