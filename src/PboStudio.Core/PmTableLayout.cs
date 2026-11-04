namespace PboStudio.Core;

/// <summary>
/// Where the live readings sit in the power monitoring table the SMU publishes into DRAM.
///
/// The table's layout is different for every processor generation, and the program used to read
/// one fixed set of positions - the Zen 3 ones - on everything. On a Zen 4 part those positions
/// hold something else, which came out as 0 °C and 0 W: the symptom somebody reported on a
/// 7950X3D. Worse, the emergency temperature cut-off compares against the same reading, so a
/// processor whose table was misread had no thermal protection and nobody was told.
/// </summary>
/// <param name="Name">Which layout this is, for the log.</param>
/// <param name="Verified">
/// Whether the positions have been checked against a real processor. A layout that is not
/// verified is still used - the alternative is showing nothing - but it is said to be a guess.
/// </param>
public sealed record PmTableLayout(
    string Name,
    bool Verified,
    int? PptLimit, int? PptNow,
    int? TdcLimit, int? TdcNow,
    int? EdcLimit, int? EdcNow,
    int? TempLimit, int? TempNow)
{
    /// <summary>
    /// Zen 3 (Vermeer), table versions 0x3808xx and 0x3809xx. Checked on a Ryzen 7 5800X3D:
    /// every figure matches what the BIOS and other monitoring tools report.
    /// </summary>
    public static readonly PmTableLayout Zen3 = new(
        "Zen 3", Verified: true,
        PptLimit: 0, PptNow: 1,
        TdcLimit: 2, TdcNow: 3,
        EdcLimit: 8, EdcNow: 9,
        TempLimit: 4, TempNow: 5);

    /// <summary>
    /// Zen 4 (Raphael), table version 0x540004. The positions come from the sensor definitions
    /// in the vendored ZenStates-Core, which its own author maintains - not from a measurement
    /// made here, because no Zen 4 processor was available. The limits are not defined there, so
    /// they are left out rather than invented: the readings show without a bar.
    /// </summary>
    public static readonly PmTableLayout Zen4 = new(
        "Zen 4", Verified: false,
        PptLimit: null, PptNow: 3,
        TdcLimit: null, TdcNow: 48,
        EdcLimit: null, EdcNow: 49,
        TempLimit: null, TempNow: 11);

    /// <summary>
    /// The layout for a table version, or null when it is not known. Null is the point: the
    /// old behaviour of reading the Zen 3 positions anyway is what produced the zeros.
    /// </summary>
    public static PmTableLayout? For(uint tableVersion) => tableVersion switch
    {
        _ when (tableVersion & 0xFFFF0000u) == 0x00380000u => Zen3,
        0x00540004u => Zen4,
        _ => null,
    };

    /// <summary>The largest index this layout reads, so a short table can be rejected whole.</summary>
    public int HighestIndex => new[] { PptLimit, PptNow, TdcLimit, TdcNow, EdcLimit, EdcNow, TempLimit, TempNow }
        .Max(i => i ?? -1);

    /// <summary>Power and current: finite and positive, and nowhere near a mis-aligned float.</summary>
    public static float? Reading(float v) => float.IsFinite(v) && v > 0 && v < 10000 ? v : null;

    /// <summary>
    /// A temperature has a far narrower plausible range than any other figure, so it gets its own
    /// check. A package temperature outside it is a misread table, not a hot processor, and
    /// acting on it - cutting a run short, or worse, trusting a low figure - is worse than
    /// reporting nothing.
    /// </summary>
    public static float? Temperature(float v) => float.IsFinite(v) && v is >= 1f and <= 125f ? v : null;

    /// <summary>Pulls the readings out of a raw table, or null when the table is too short for the layout.</summary>
    public PboSnapshot? Read(float[] table, float scalar, uint boostMhz)
    {
        if (table.Length <= HighestIndex) return null;

        float? At(int? index, Func<float, float?> check) => index is { } i ? check(table[i]) : null;

        return new PboSnapshot(scalar, boostMhz,
            PptLimitW: At(PptLimit, Reading), PptNowW: At(PptNow, Reading),
            TdcLimitA: At(TdcLimit, Reading), TdcNowA: At(TdcNow, Reading),
            EdcLimitA: At(EdcLimit, Reading), EdcNowA: At(EdcNow, Reading),
            TempLimitC: At(TempLimit, Temperature), TempNowC: At(TempNow, Temperature));
    }
}
