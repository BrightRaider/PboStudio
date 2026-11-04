using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

/// <summary>
/// A machine check names the processor by APIC ID, and the program treated the ID as the logical
/// processor's number. True on the part it was checked on, an assumption elsewhere: with cores
/// fused off the IDs can skip numbers. A genuine fault on the core under test would then be
/// credited to another core, reported as "not counted against this core", and the core would pass.
/// </summary>
public class ApicTopologyTests(Xunit.Abstractions.ITestOutputHelper output)
{
    /// <summary>Twelve cores with SMT: core N owns logical processors 2N and 2N+1.</summary>
    private static IReadOnlyList<PhysicalCore> TwelveCores() =>
        [.. Enumerable.Range(0, 12).Select(i => new PhysicalCore(i, (nuint)3 << (i * 2), [i * 2, i * 2 + 1]))];

    /// <summary>
    /// What a 6+6 part can report: IDs 0-11 on the first chiplet, then a gap, then 16-27. Logical
    /// processor 12 has APIC ID 16.
    /// </summary>
    private static IReadOnlyDictionary<int, int> GappedMap() =>
        Enumerable.Range(0, 24).ToDictionary(lp => lp < 12 ? lp : lp + 4, lp => lp);

    // -- the verified case must not change -----------------------------------------------

    /// <summary>The Ryzen 7 5800X3D: APIC 9 was logical processor 9, which is core 4.</summary>
    [Fact]
    public void WithoutAMapTheOldAssumptionStillHolds()
    {
        var cores = Enumerable.Range(0, 8)
            .Select(i => new PhysicalCore(i, (nuint)3 << (i * 2), [i * 2, i * 2 + 1])).ToList();

        Assert.Equal(4, ApicTopology.CoreForApicId(9, cores, null));
        Assert.Equal(0, ApicTopology.CoreForApicId(0, cores, null));
    }

    [Fact]
    public void AnIdentityMapGivesTheSameAnswerAsNoMap()
    {
        var cores = Enumerable.Range(0, 8)
            .Select(i => new PhysicalCore(i, (nuint)3 << (i * 2), [i * 2, i * 2 + 1])).ToList();
        var identity = Enumerable.Range(0, 16).ToDictionary(i => i, i => i);

        for (int apic = 0; apic < 16; apic++)
            Assert.Equal(
                ApicTopology.CoreForApicId(apic, cores, null),
                ApicTopology.CoreForApicId(apic, cores, identity));
    }

    // -- the case that was wrong ---------------------------------------------------------

    /// <summary>
    /// APIC 16 is logical processor 12 on this part, which is core 6. The old rule read it as
    /// logical processor 16, which is core 8.
    /// </summary>
    [Fact]
    public void AGapInTheIdsIsFollowedRatherThanAssumedAway()
    {
        var cores = TwelveCores();

        Assert.Equal(8, ApicTopology.CoreForApicId(16, cores, null));          // the old, wrong answer
        Assert.Equal(6, ApicTopology.CoreForApicId(16, cores, GappedMap()));   // the right one
    }

    [Fact]
    public void BothSiblingsOfACoreResolveToIt()
    {
        var cores = TwelveCores();
        var map = GappedMap();

        Assert.Equal(6, ApicTopology.CoreForApicId(16, cores, map));
        Assert.Equal(6, ApicTopology.CoreForApicId(17, cores, map));
    }

    /// <summary>An ID that is in the gap belongs to no processor, so it blames no core.</summary>
    [Fact]
    public void AnIdThatNoProcessorOwnsBlamesNoCore()
    {
        Assert.Null(ApicTopology.CoreForApicId(13, TwelveCores(), GappedMap()));
    }

    [Fact]
    public void AnIdOutsideEveryCoreBlamesNoCore()
    {
        Assert.Null(ApicTopology.CoreForApicId(500, TwelveCores(), null));
    }

    // -- reading it from the processor ----------------------------------------------------

    /// <summary>
    /// Run for real on whatever this machine is. It is not a statement about the machine, only
    /// that the reading is internally consistent: unique IDs, one per logical processor.
    /// </summary>
    [Fact]
    public void ReadingTheRealProcessorGivesOneUniqueIdPerLogicalProcessor()
    {
        int logical = Environment.ProcessorCount;
        var map = ApicTopology.Read(logical);

        // Null is allowed (not x86, or pinning refused): the contract is "map or nothing".
        output.WriteLine(map is null
            ? "APIC map: not readable on this machine"
            : "APIC map (apic -> logical): " + string.Join(", ", map.OrderBy(kv => kv.Value).Select(kv => $"{kv.Key}->{kv.Value}")));
        if (map is null) return;

        Assert.Equal(logical, map.Count);
        Assert.Equal(Enumerable.Range(0, logical), map.Values.Order());
    }
}

/// <summary>
/// The crash check took the twenty newest WHEA records of any kind. A machine that logs a few
/// dozen corrected errors after a crash pushes the crash out of that window, and it is never
/// offered. The fatal ones are asked for directly now.
/// </summary>
public class FatalWheaQueryTests(Xunit.Abstractions.ITestOutputHelper output)
{
    [Fact]
    public void TheFatalQueryReturnsOnlyFatalEventsAndNewestFirst()
    {
        var fatal = WheaWatcher.ReadFatal(50);
        var all = WheaWatcher.ReadExisting(20);

        output.WriteLine($"fatal: {fatal.Count}, any (newest 20): {all.Count}, " +
                         $"fatal among those 20: {all.Count(e => e.EventId == 18)}");

        Assert.All(fatal, e => Assert.Equal(18, e.EventId));
        Assert.Equal(fatal.OrderByDescending(e => e.Time), fatal);
    }

    /// <summary>
    /// What the old window could miss: every fatal event found by the targeted query must be a
    /// superset of those visible in the newest twenty of any kind.
    /// </summary>
    [Fact]
    public void TheTargetedQueryNeverFindsFewerFatalEventsThanTheCrowdedWindow()
    {
        int inWindow = WheaWatcher.ReadExisting(20).Count(e => e.EventId == 18);

        Assert.True(WheaWatcher.ReadFatal(50).Count >= inWindow);
    }
}
