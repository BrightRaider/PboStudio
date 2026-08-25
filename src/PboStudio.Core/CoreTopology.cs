using System.Runtime.InteropServices;

namespace PboStudio.Core;

/// <summary>One physical core and the logical processors that live on it.</summary>
public sealed record PhysicalCore(
    int Index,
    nuint AffinityMask,
    int[] LogicalProcessors,
    /// <summary>
    /// Index of the last-level cache this core shares. On Ryzen one L3 slice is exactly one
    /// CCX, which is the finest chiplet boundary Windows will admit to.
    /// </summary>
    int L3Group = 0,
    /// <summary>
    /// Index of the processor die Windows reports. Most AMD firmware reports a single die for
    /// the whole package, so this is usually 0 and the cache view has to do the work.
    /// </summary>
    int Die = 0)
{
    public bool HasSmt => LogicalProcessors.Length > 1;

    /// <summary>Affinity mask covering only the first logical processor of this core.</summary>
    public nuint FirstThreadMask => (nuint)1 << LogicalProcessors[0];

    /// <summary>
    /// The mask a session should run under.
    /// <para>
    /// <paramref name="spreadAcrossSmt"/> is the third combination: one worker, but free to move
    /// between both SMT siblings. The scheduler then shifts the load back and forth, which
    /// produces transitions that neither a hard pin nor two saturated threads ever create.
    /// </para>
    /// </summary>
    public nuint MaskFor(int threads, bool spreadAcrossSmt = false) =>
        threads >= 2 || (spreadAcrossSmt && HasSmt) ? AffinityMask : FirstThreadMask;

    public override string ToString() =>
        $"Core {Index} (CPU {string.Join("+", LogicalProcessors)}, mask 0x{AffinityMask:X}, L3 group {L3Group}, die {Die})";
}

/// <summary>Which chiplet every core sits on, and how many chiplets there are.</summary>
public sealed record CcdLayout(int CcdCount, IReadOnlyDictionary<int, int> CoreToCcd)
{
    public bool IsMultiCcd => CcdCount > 1;

    public int CcdFor(int coreIndex) => CoreToCcd.TryGetValue(coreIndex, out int ccd) ? ccd : 0;

    public IEnumerable<int> CoresOn(int ccd) =>
        CoreToCcd.Where(kv => kv.Value == ccd).Select(kv => kv.Key).Order();

    public static CcdLayout Single(IEnumerable<int> coreIndices) =>
        new(1, coreIndices.ToDictionary(i => i, _ => 0));
}

/// <summary>
/// Physical core layout as Windows reports it. Deliberately not derived from processor
/// numbering: hybrid CPUs interleave performance and efficiency cores, so the common
/// "core N owns processors 2N and 2N+1" shortcut produces wrong affinity masks there.
/// </summary>
public static class CoreTopology
{
    private const int RelationProcessorCore = 0;
    private const int RelationCache = 2;
    private const int RelationProcessorDie = 5;
    private const int RelationAll = 0xffff;

    public static IReadOnlyList<PhysicalCore> Enumerate()
    {
        uint length = 0;
        Native.GetLogicalProcessorInformationEx(RelationAll, nint.Zero, ref length);
        if (length == 0)
            throw new InvalidOperationException(
                $"GetLogicalProcessorInformationEx returned no size: {Marshal.GetLastWin32Error()}");

        nint buffer = Marshal.AllocHGlobal((int)length);
        try
        {
            if (!Native.GetLogicalProcessorInformationEx(RelationAll, buffer, ref length))
                throw new InvalidOperationException(
                    $"GetLogicalProcessorInformationEx failed: {Marshal.GetLastWin32Error()}");

            var coreMasks = new List<(nuint Mask, List<int> Logical)>();
            var dieMasks = new List<List<int>>();

            // Cache records arrive for every level; only the deepest one describes the chiplet.
            byte deepestCacheLevel = 0;
            var cacheMasks = new List<(byte Level, List<int> Logical)>();

            nint p = buffer;
            nint end = buffer + (int)length;

            while (p < end)
            {
                int relationship = Marshal.ReadInt32(p, 0);
                int size = Marshal.ReadInt32(p, 4);
                // A non-positive size would loop forever; bail rather than hang.
                if (size <= 0) break;

                switch (relationship)
                {
                    // SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX: Relationship(4) Size(4), then
                    // PROCESSOR_RELATIONSHIP: Flags(1) EfficiencyClass(1) Reserved(20)
                    // GroupCount(2) GroupMask[] of GROUP_AFFINITY: Mask(8) Group(2) Reserved(6)
                    case RelationProcessorCore:
                    {
                        var (mask, logical) = ReadGroupAffinities(p, countOffset: 30, masksOffset: 32);
                        if (logical.Count > 0) coreMasks.Add((mask, logical));
                        break;
                    }

                    // RelationProcessorDie shares PROCESSOR_RELATIONSHIP with RelationProcessorCore.
                    case RelationProcessorDie:
                    {
                        var (_, logical) = ReadGroupAffinities(p, countOffset: 30, masksOffset: 32);
                        if (logical.Count > 0) dieMasks.Add(logical);
                        break;
                    }

                    // CACHE_RELATIONSHIP: Level(1) Associativity(1) LineSize(2) CacheSize(4)
                    // Type(4) Reserved(18) GroupCount(2) GroupMask[]. GroupCount only exists
                    // from Windows 10 20H2; before that those bytes are reserved and read 0,
                    // and the single GroupMask sits at the same offset either way.
                    case RelationCache:
                    {
                        byte level = Marshal.ReadByte(p, 8);
                        var (_, logical) = ReadGroupAffinities(p, countOffset: 38, masksOffset: 40);
                        if (logical.Count == 0) break;

                        cacheMasks.Add((level, logical));
                        if (level > deepestCacheLevel) deepestCacheLevel = level;
                        break;
                    }
                }

                p += size;
            }

            var llcGroups = OrderGroups(cacheMasks.Where(c => c.Level == deepestCacheLevel).Select(c => c.Logical));
            var dieGroups = OrderGroups(dieMasks);

            var cores = new List<PhysicalCore>(coreMasks.Count);
            for (int i = 0; i < coreMasks.Count; i++)
            {
                var (mask, logical) = coreMasks[i];
                int first = logical[0];
                cores.Add(new PhysicalCore(
                    i, mask, [.. logical],
                    L3Group: IndexOfGroupContaining(llcGroups, first),
                    Die: IndexOfGroupContaining(dieGroups, first)));
            }

            return cores;
        }
        finally
        {
            Marshal.FreeHGlobal(buffer);
        }
    }

    private static (nuint Mask, List<int> Logical) ReadGroupAffinities(nint record, int countOffset, int masksOffset)
    {
        ushort groupCount = (ushort)Marshal.ReadInt16(record, countOffset);
        // Pre-20H2 cache records have no count field; the union still holds one GROUP_AFFINITY.
        if (groupCount == 0) groupCount = 1;

        nuint combined = 0;
        var logical = new List<int>();

        for (int g = 0; g < groupCount; g++)
        {
            nuint mask = (nuint)(ulong)Marshal.ReadInt64(record, masksOffset + g * 16);
            combined |= mask;

            for (int bit = 0; bit < nuint.Size * 8; bit++)
                if ((mask & ((nuint)1 << bit)) != 0)
                    logical.Add(bit);
        }

        return (combined, logical);
    }

    /// <summary>Sorted by lowest processor number, so group 0 is always the one holding CPU 0.</summary>
    private static List<HashSet<int>> OrderGroups(IEnumerable<List<int>> groups) =>
        [.. groups.Where(g => g.Count > 0).OrderBy(g => g.Min()).Select(g => new HashSet<int>(g))];

    private static int IndexOfGroupContaining(List<HashSet<int>> groups, int logicalProcessor)
    {
        for (int i = 0; i < groups.Count; i++)
            if (groups[i].Contains(logicalProcessor))
                return i;
        return 0;
    }

    // Zen 3 and newer put one CCX on one CCD; Zen and Zen 2 put two. A CCX never holds more
    // than four cores on the two-per-CCD parts, which is what makes the fallback below safe.
    private const int MaxCoresPerCcxOnSplitCcdParts = 4;

    /// <summary>
    /// Maps cores to chiplets from what the OS actually reports, preferring the most direct
    /// evidence available.
    /// <para>
    /// 1. If Windows enumerates more than one processor die, that is the answer outright.
    /// 2. Otherwise fall back to last-level-cache groups. On Ryzen an L3 slice is one CCX,
    ///    which equals one CCD from Zen 3 onward but is half a CCD on Zen and Zen 2.
    /// 3. <paramref name="smuCcdCount"/> comes from the CPU's own CCD fuse and settles that
    ///    ambiguity when the driver could read it; without it, CCX size stands in.
    /// </para>
    /// </summary>
    public static CcdLayout ResolveCcds(IReadOnlyList<PhysicalCore> cores, int? smuCcdCount = null)
    {
        if (cores.Count == 0) return new CcdLayout(1, new Dictionary<int, int>());

        var dieIndices = cores.Select(c => c.Die).Distinct().Order().ToList();
        if (dieIndices.Count > 1)
        {
            return new CcdLayout(
                dieIndices.Count,
                cores.ToDictionary(c => c.Index, c => dieIndices.IndexOf(c.Die)));
        }

        var ccxIndices = cores.Select(c => c.L3Group).Distinct().Order().ToList();
        int ccxCount = ccxIndices.Count;
        if (ccxCount <= 1) return CcdLayout.Single(cores.Select(c => c.Index));

        int ccxPerCcd = 1;

        if (smuCcdCount is > 0 && smuCcdCount < ccxCount && ccxCount % smuCcdCount == 0)
        {
            ccxPerCcd = ccxCount / smuCcdCount.Value;
        }
        else if (smuCcdCount is null or <= 0)
        {
            // No fuse reading available. Two CCXs share a CCD only on the parts that cap a CCX
            // at four cores, so a small, even CCX count is the tell.
            int largestCcx = ccxIndices.Max(ccx => cores.Count(c => c.L3Group == ccx));
            if (ccxCount % 2 == 0 && largestCcx <= MaxCoresPerCcxOnSplitCcdParts)
                ccxPerCcd = 2;
        }

        int ccdCount = ccxCount / ccxPerCcd;
        var map = cores.ToDictionary(
            c => c.Index,
            c => ccxIndices.IndexOf(c.L3Group) / ccxPerCcd);

        return new CcdLayout(ccdCount, map);
    }
}
