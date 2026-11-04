namespace PboStudio.Core;

/// <summary>Where the SMU addresses one core: which chiplet, and which of its eight physical slots.</summary>
public sealed record SmuCoreSlot(int Ccd, int Slot);

/// <summary>
/// Maps the cores Windows enumerates to the (chiplet, slot) pairs the SMU addresses.
///
/// Each chiplet has eight physical core slots, and a part with fewer cores has the rest fused
/// off. Windows lists only the enabled cores, in order; the SMU is addressed by physical slot.
/// The mapping the program used was "core N is slot N modulo the cores per chiplet", with the
/// cores per chiplet taken from a figure that is really eight slots per chiplet. It holds for an
/// eight-core part and for a 16-core one, and fails on every 6+6 and every 6-core part:
///
///  - On a 12-core processor (7900X3D, 7900X, 5900X) the divisor was 8 instead of 6, so Windows
///    cores 6 to 11 - the whole second chiplet - were written to the wrong chiplet and slot.
///  - On a part whose fused-off slots are not the top ones, "slot N" can be a disabled core.
///
/// The consequence is the worst kind: a value is written to some core, the core under test passes
/// at a value it never held, and that value ends up in the BIOS list. The fuse map the processor
/// reports says which slots are enabled; the k-th enabled slot of a chiplet is the k-th core
/// Windows lists for it.
/// </summary>
public static class SmuCoreMap
{
    public const int SlotsPerCcd = 8;

    /// <param name="disabledSlotsPerCcd">
    /// One mask per chiplet, bit N set meaning slot N is fused off. As read from the processor.
    /// </param>
    /// <param name="windowsCoreCount">How many physical cores Windows reports.</param>
    /// <returns>
    /// A slot per core, in core order - or null when the fuse map and Windows disagree about how
    /// many cores there are. Null is the point: a mapping that does not add up is worse than the
    /// caller's fallback, and the caller says so.
    /// </returns>
    public static IReadOnlyList<SmuCoreSlot>? Build(IReadOnlyList<uint>? disabledSlotsPerCcd, int windowsCoreCount)
    {
        if (disabledSlotsPerCcd is not { Count: > 0 } || windowsCoreCount <= 0) return null;

        var map = new List<SmuCoreSlot>(windowsCoreCount);

        for (int ccd = 0; ccd < disabledSlotsPerCcd.Count; ccd++)
        {
            uint disabled = disabledSlotsPerCcd[ccd];

            for (int slot = 0; slot < SlotsPerCcd; slot++)
                if ((disabled & (1u << slot)) == 0)
                    map.Add(new SmuCoreSlot(ccd, slot));
        }

        return map.Count == windowsCoreCount ? map : null;
    }

    /// <summary>
    /// The fallback when no fuse map is available: assume the fused-off slots are the top ones,
    /// and divide the cores Windows reports evenly between the chiplets. Correct for every part
    /// whose chiplets are fully populated, and for the common 6-core-per-chiplet case; wrong only
    /// where a lower slot is disabled, which is exactly when the fuse map would have been needed.
    /// </summary>
    public static SmuCoreSlot? Fallback(int core, int ccdCount, int windowsCoreCount)
    {
        if (core < 0 || ccdCount <= 0 || windowsCoreCount <= 0 || windowsCoreCount % ccdCount != 0)
            return null;

        int perCcd = windowsCoreCount / ccdCount;
        return core < windowsCoreCount ? new SmuCoreSlot(core / perCcd, core % perCcd) : null;
    }
}
