using System.Runtime.Intrinsics.X86;

namespace PboStudio.Core;

/// <summary>
/// Which logical processor owns which APIC ID.
///
/// A machine check names the processor that raised it by APIC ID, and the program turned that
/// into a core by treating the ID as the logical processor's number. That is true on the part it
/// was checked on - a Ryzen 7 5800X3D reported APIC 9 for a fault on logical processor 9 - and an
/// assumption everywhere else. On a processor with cores fused off, the IDs can skip numbers, and
/// then "APIC 16" is not logical processor 16. The mistake runs in the dangerous direction: a
/// genuine fault on the core under test is credited to a different core, reported as "not counted
/// against this core", and the core passes.
///
/// The processor will say what its IDs are. CPUID leaf 0xB returns the x2APIC ID of whichever
/// logical processor executes it, so running it once on each is a direct reading, not a guess.
/// </summary>
public static class ApicTopology
{
    /// <summary>
    /// APIC ID to logical processor number, or null when it could not be read. Null makes callers
    /// fall back to the old assumption rather than refuse to attribute anything.
    /// </summary>
    public static IReadOnlyDictionary<int, int>? Read(int logicalProcessorCount)
    {
        if (!OperatingSystem.IsWindows() || !X86Base.IsSupported) return null;
        if (logicalProcessorCount <= 0 || logicalProcessorCount > nuint.Size * 8) return null;

        IReadOnlyDictionary<int, int>? result = null;

        // Its own thread: pinning moves it, and the calling thread may be the UI thread.
        var worker = new Thread(() => result = ReadPinned(logicalProcessorCount)) { IsBackground = true };
        worker.Start();
        worker.Join(TimeSpan.FromSeconds(5));

        return result;
    }

    private static Dictionary<int, int>? ReadPinned(int count)
    {
        nint thread = Native.GetCurrentThread();
        nuint original = 0;

        try
        {
            var map = new Dictionary<int, int>(count);

            for (int lp = 0; lp < count; lp++)
            {
                nuint previous = Native.SetThreadAffinityMask(thread, (nuint)1 << lp);
                if (previous == 0) return null;   // could not pin: do not trust anything read
                if (original == 0) original = previous;

                // Subleaf 1 is the core level; EDX is the x2APIC ID in every subleaf.
                int apic = X86Base.CpuId(0x0B, 1).Edx;
                if (!map.TryAdd(apic, lp)) return null;   // two processors claiming one ID is not a map
            }

            return map;
        }
        catch
        {
            return null;
        }
        finally
        {
            if (original != 0) Native.SetThreadAffinityMask(thread, original);
        }
    }

    /// <summary>
    /// The core owning an APIC ID. With a measured map the answer is exact; without one it falls
    /// back to treating the ID as the logical processor number.
    /// </summary>
    public static int? CoreForApicId(
        int apicId, IReadOnlyList<PhysicalCore> cores, IReadOnlyDictionary<int, int>? apicToLogical)
    {
        int logical;

        if (apicToLogical is not null)
        {
            if (!apicToLogical.TryGetValue(apicId, out logical)) return null;
        }
        else
        {
            logical = apicId;
        }

        foreach (var core in cores)
            if (core.LogicalProcessors.Contains(logical)) return core.Index;

        return null;
    }
}
