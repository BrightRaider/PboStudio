using System.Security.Principal;
using PboStudio.Core;
using ZenStates.Core;

// Diagnostic console for the SMU path.
//   (no args)  read-only dump
//   write      write a Curve Optimizer margin to one core and read it back
//   topo       physical core layout, no elevation needed
//
// The write test exists to answer one question: does GetDldoPsmMargin report the value the
// BIOS applied at boot, or only what was written through the SMU at runtime?

if (args.Length > 0 && args[0].Equals("topo", StringComparison.OrdinalIgnoreCase))
{
    foreach (var core in CoreTopology.Enumerate())
        Console.WriteLine(core);
    return 0;
}

if (args.Length > 0 && args[0].Equals("yc", StringComparison.OrdinalIgnoreCase))
{
    int coreIndex = args.Length > 1 ? int.Parse(args[1]) : 0;
    int seconds = args.Length > 2 ? int.Parse(args[2]) : 20;
    int threads = args.Length > 3 ? int.Parse(args[3]) : 1;

    string ycRoot = Path.GetFullPath(Path.Combine(
        AppContext.BaseDirectory, "..", "..", "..", "..", "..", "external", "engines", "ycruncher"));
    string? install = Directory.EnumerateDirectories(ycRoot, "y-cruncher*").FirstOrDefault();
    if (install is null) { Console.Error.WriteLine("y-cruncher not found."); return 1; }

    string? binary = YCruncherEngine.FindBinaryFor(install, ZenGeneration.Zen3);
    if (binary is null) { Console.Error.WriteLine("No suitable binary."); return 1; }
    Console.WriteLine($"Binary: {Path.GetFileName(binary)}");

    var ycEngine = new YCruncherEngine(binary, Path.Combine(ycRoot, "runs"),
        new YCruncherOptions(["VT3", "FFT", "N63"], SecondsPerTest: 10));

    var ycTarget = CoreTopology.Enumerate()[coreIndex];
    Console.WriteLine($"Loading {ycTarget} with {threads} thread(s) for {seconds}s");

    using var ycSession = ycEngine.Start(ycTarget, threads);
    var ycStart = ycSession.CpuTime;
    var ycClock = System.Diagnostics.Stopwatch.StartNew();

    while (ycClock.Elapsed.TotalSeconds < seconds)
    {
        Thread.Sleep(1000);
        foreach (var f in ycSession.DrainFailures()) Console.WriteLine($"  FAILURE {f}");
        if (!ycSession.IsAlive) break;
    }

    double ycBusy = (ycSession.CpuTime - ycStart).TotalSeconds;
    Console.WriteLine($"Used {ycBusy:F1}s CPU over {ycClock.Elapsed.TotalSeconds:F1}s wall " +
                      $"= {ycBusy / ycClock.Elapsed.TotalSeconds:F2} cores");
    ycSession.Stop();
    return 0;
}

if (args.Length > 0 && args[0].Equals("stress", StringComparison.OrdinalIgnoreCase))
{
    int coreIndex = args.Length > 1 ? int.Parse(args[1]) : 0;
    int seconds = args.Length > 2 ? int.Parse(args[2]) : 20;
    int threads = args.Length > 3 ? int.Parse(args[3]) : 2;

    string engineRoot = Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "..", "..", "external", "engines");
    var engine = new Prime95Engine(
        Path.GetFullPath(Path.Combine(engineRoot, "prime95", "prime95.exe")),
        Path.GetFullPath(Path.Combine(engineRoot, "runs")));

    if (!engine.IsAvailable) { Console.Error.WriteLine("Prime95 not found."); return 1; }

    var target = CoreTopology.Enumerate()[coreIndex];
    Console.WriteLine($"Loading {target} with {threads} thread(s) for {seconds}s");

    using var session = engine.Start(target, threads);
    var started = session.CpuTime;
    var clock = System.Diagnostics.Stopwatch.StartNew();

    while (clock.Elapsed.TotalSeconds < seconds)
    {
        Thread.Sleep(1000);
        foreach (var f in session.DrainFailures()) Console.WriteLine($"  FAILURE {f}");
        if (!session.IsAlive) break;
    }

    double busy = (session.CpuTime - started).TotalSeconds;
    Console.WriteLine($"Used {busy:F1}s CPU over {clock.Elapsed.TotalSeconds:F1}s wall = {busy / clock.Elapsed.TotalSeconds:F2} cores");
    session.Stop();
    return 0;
}

if (args.Length > 0 && args[0].Equals("run", StringComparison.OrdinalIgnoreCase))
{
    int secondsPerCore = args.Length > 1 ? int.Parse(args[1]) : 10;

    string engineRoot = Path.GetFullPath(Path.Combine(
        AppContext.BaseDirectory, "..", "..", "..", "..", "..", "external", "engines"));
    var engine = new Prime95Engine(
        Path.Combine(engineRoot, "prime95", "prime95.exe"),
        Path.Combine(engineRoot, "runs"));

    var plan = new TestPlan
    {
        RuntimePerCore = TimeSpan.FromSeconds(secondsPerCore),
        Threads = 1,
        MaxIterations = 1,
        Order = CoreOrder.Alternate,
        DelayBetweenCores = TimeSpan.FromSeconds(1),
    };

    var runner = new TestRunner(engine, CoreTopology.Enumerate(), plan);
    runner.Progress += e => Console.WriteLine(e switch
    {
        TestEvent.RunStarted s => $"Run: {s.CoreCount} cores, {s.PerCore.TotalSeconds:F0}s each",
        TestEvent.CoreStarted s => $"  core {s.Core}: testing...",
        TestEvent.CorePassed s => $"  core {s.Core}: passed",
        TestEvent.CoreFailed s => $"  core {s.Core}: FAILED - {s.Failure}",
        TestEvent.Info s => $"  {s.Message}",
        TestEvent.RunFinished s => $"Done. {s.Failures.Count} core(s) with failures.",
        _ => e.ToString() ?? "",
    });

    await runner.RunAsync(CancellationToken.None);
    return 0;
}

if (args.Length > 0 && args[0].Equals("whea", StringComparison.OrdinalIgnoreCase))
{
    var past = WheaWatcher.ReadExisting();
    Console.WriteLine(past.Count == 0
        ? "No machine check events in the System log."
        : $"{past.Count} machine check event(s), newest first:");
    foreach (var e in past) Console.WriteLine("  " + e);
    return 0;
}

using var id = WindowsIdentity.GetCurrent();
if (!new WindowsPrincipal(id).IsInRole(WindowsBuiltInRole.Administrator))
{
    Console.Error.WriteLine("Needs to run elevated. SMU access goes through a kernel driver.");
    return 2;
}

Cpu cpu;
try
{
    cpu = new Cpu();
}
catch (Exception ex)
{
    Console.Error.WriteLine($"Cpu init failed: {ex.GetType().Name}: {ex.Message}");
    return 1;
}

var info = cpu.info;
var topo = info.topology;
uint coreCount = topo.physicalCores;
uint ccds = Math.Max(1u, topo.ccds);
uint coresPerCcd = coreCount / ccds;

Console.WriteLine($"CPU        : {info.cpuName}");
Console.WriteLine($"Codename   : {info.codeName}   Family: {info.family}");
Console.WriteLine($"SMU type   : {cpu.smu.SMU_TYPE}   SMU ver: {cpu.smu.Version:X8}");
Console.WriteLine($"Topology   : {ccds} CCD / {topo.ccxs} CCX / {coreCount} cores / {topo.logicalCores} threads");
Console.WriteLine();

if (args.Length > 0 && args[0].Equals("pmtable", StringComparison.OrdinalIgnoreCase))
{
    var version = cpu.GetTableVersion();
    Console.WriteLine($"Table version : 0x{version.TableVersion:X8}   size: {version.TableSize}");
    Console.WriteLine($"DRAM address  : 0x{cpu.GetDramBaseAddress():X8} / 0x{cpu.GetDramBaseAddress64():X16}");
    Console.WriteLine($"powerTable    : {(cpu.powerTable is null ? "null" : "created")}");
    if (cpu.powerTable is not null)
        Console.WriteLine($"  its address : 0x{cpu.powerTable.DramBaseAddress:X16}");

    var status = cpu.RefreshPowerTable();
    var table = cpu.powerTable?.Table;

    Console.WriteLine($"Refresh: {status}   entries: {table?.Length ?? 0}");
    if (table is null || table.Length == 0) return 1;

    // Hunt for the fused limits by value. PPT/TDC/EDC are the only entries that sit at these
    // magnitudes, so matching on the known figures identifies the offsets without a table map.
    Console.WriteLine("\nCandidates (value -> index):");
    for (int i = 0; i < table.Length; i++)
    {
        float v = table[i];
        if (float.IsNaN(v) || v <= 0) continue;

        string? tag = v switch
        {
            >= 139 and <= 146 => "PPT?",
            >= 93 and <= 97 => "TDC?",
            >= 127 and <= 133 => "EDC?",
            _ => null,
        };
        if (tag is not null)
            Console.WriteLine($"  [{i,3}] {v,10:F3}   {tag}");
    }

    Console.WriteLine("\nFirst 64 non-zero entries:");
    for (int i = 0, shown = 0; i < table.Length && shown < 64; i++)
    {
        if (table[i] is 0 or float.NaN) continue;
        Console.WriteLine($"  [{i,3}] {table[i],12:F3}");
        shown++;
    }
    return 0;
}

bool writeMode = args.Length > 0 && args[0].Equals("write", StringComparison.OrdinalIgnoreCase);

if (!writeMode)
{
    DumpCurveOptimizer();
    DumpPbo();
    return 0;
}

// --- write test -----------------------------------------------------------------------
const int TestMargin = -5;
const uint TestCore = 0;

uint testMask = cpu.MakeCoreMask(TestCore, 0, 0);

Console.WriteLine($"Write test on core {TestCore} (mask 0x{testMask:X8})");
Console.WriteLine($"  before        : {Show(cpu.GetPsmMarginSingleCore(testMask))}");

bool ok = cpu.SetPsmMarginSingleCore(testMask, TestMargin);
Console.WriteLine($"  set {TestMargin,3}       : {(ok ? "accepted" : "REJECTED")}");
Thread.Sleep(250);

uint? after = cpu.GetPsmMarginSingleCore(testMask);
Console.WriteLine($"  read back     : {Show(after)}");

bool restored = cpu.SetPsmMarginSingleCore(testMask, 0);
Thread.Sleep(250);
Console.WriteLine($"  restored to 0 : {(restored ? "accepted" : "REJECTED")} -> {Show(cpu.GetPsmMarginSingleCore(testMask))}");

Console.WriteLine();
if (after is { } a && unchecked((int)a) == TestMargin)
    Console.WriteLine("VERDICT: read path works, but only reflects runtime SMU writes.");
else if (!ok)
    Console.WriteLine("VERDICT: this CPU refuses Curve Optimizer writes through the SMU.");
else
    Console.WriteLine("VERDICT: write accepted but read stays stale - read path is unusable here.");

Console.WriteLine();
Console.WriteLine("Core 0 is now at 0 regardless of what the BIOS applied. Reboot to restore it.");
return 0;

string Show(uint? v) => v is null ? "-- no response --" : $"{unchecked((int)v.Value),4}  (raw 0x{v.Value:X8})";

void DumpCurveOptimizer()
{
    Console.WriteLine("Curve Optimizer (read-only):");
    int idx = 0;
    for (uint ccd = 0; ccd < ccds; ccd++)
    {
        for (uint c = 0; c < coresPerCcd; c++)
        {
            uint mask = cpu.MakeCoreMask(c, ccd, 0);
            Console.WriteLine($"  Core {idx,2} [ccd {ccd} mask 0x{mask:X8}] : {Show(cpu.GetPsmMarginSingleCore(mask))}");
            idx++;
        }
    }
    Console.WriteLine();
}

void DumpPbo()
{
    Console.WriteLine("PBO:");
    try { Console.WriteLine($"  Scalar        : {cpu.GetPBOScalar()}x"); }
    catch (Exception ex) { Console.WriteLine($"  Scalar        : failed ({ex.GetType().Name})"); }

    try
    {
        if (cpu.GetPboFusedLimits() is { } l)
        {
            Console.WriteLine($"  PPT (power)   : {l.PowerLimit} W");
            Console.WriteLine($"  TDC (slow)    : {l.SlowLimit} A");
            Console.WriteLine($"  EDC (fast)    : {l.FastLimit} A");
        }
        else Console.WriteLine("  fused limits  : -- no response --");
    }
    catch (Exception ex) { Console.WriteLine($"  fused limits  : failed ({ex.GetType().Name})"); }

    try { Console.WriteLine($"  FMax / boost  : {cpu.GetFMax()} MHz"); }
    catch (Exception ex) { Console.WriteLine($"  FMax          : failed ({ex.GetType().Name})"); }
}
