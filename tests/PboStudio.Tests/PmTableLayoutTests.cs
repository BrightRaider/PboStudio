using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

/// <summary>
/// Somebody reported 0 °C and 0 W on a Ryzen 9 7950X3D. The program read one fixed set of
/// positions out of the SMU's power table - the Zen 3 ones - on every processor, and on a Zen 4
/// part those positions hold something else.
///
/// The consequence was worse than the display: the emergency temperature cut-off compares
/// against the same reading, so a misread table meant a processor under a stress test with no
/// thermal protection, and nobody was told.
/// </summary>
public class PmTableLayoutTests
{
    /// <summary>A table of recognisable sentinel values, so a test can see which index was read.</summary>
    private static float[] Table(int length = 600)
    {
        var t = new float[length];
        for (int i = 0; i < t.Length; i++) t[i] = 0;
        return t;
    }

    // ── which layout belongs to which processor ───────────────────

    [Theory]
    [InlineData(0x380804u)]
    [InlineData(0x380805u)]
    [InlineData(0x380904u)]
    [InlineData(0x380905u)]
    public void EveryZen3TableVersionGetsTheZen3Layout(uint version)
    {
        Assert.Same(PmTableLayout.Zen3, PmTableLayout.For(version));
    }

    [Fact]
    public void Zen4GetsItsOwnLayout()
    {
        Assert.Same(PmTableLayout.Zen4, PmTableLayout.For(0x540004u));
        Assert.NotSame(PmTableLayout.Zen3, PmTableLayout.For(0x540004u));
    }

    /// <summary>
    /// The old behaviour - reading the Zen 3 positions on anything - is what produced the zeros.
    /// A table this program does not recognise has to say so rather than fall back to a guess.
    /// </summary>
    [Theory]
    [InlineData(0x620105u)]   // Zen 5
    [InlineData(0x240903u)]   // Zen 2
    [InlineData(0u)]
    public void AnUnknownTableHasNoLayoutRatherThanAGuessedOne(uint version)
    {
        Assert.Null(PmTableLayout.For(version));
    }

    [Fact]
    public void OnlyTheLayoutThatWasCheckedOnHardwareClaimsToBeVerified()
    {
        Assert.True(PmTableLayout.Zen3.Verified);
        Assert.False(PmTableLayout.Zen4.Verified);
    }

    // ── reading the readings ──────────────────────────────────────

    [Fact]
    public void Zen3ReadsTheValuesItAlwaysDid()
    {
        var t = Table();
        t[0] = 142; t[1] = 58;   // PPT limit, PPT
        t[2] = 95;  t[3] = 26;   // TDC limit, TDC
        t[4] = 90;  t[5] = 57;   // temperature limit, temperature
        t[8] = 130; t[9] = 112;  // EDC limit, EDC

        var snap = PmTableLayout.Zen3.Read(t, 1f, 4550)!;

        Assert.Equal(142, snap.PptLimitW); Assert.Equal(58, snap.PptNowW);
        Assert.Equal(95, snap.TdcLimitA);  Assert.Equal(26, snap.TdcNowA);
        Assert.Equal(130, snap.EdcLimitA); Assert.Equal(112, snap.EdcNowA);
        Assert.Equal(90, snap.TempLimitC); Assert.Equal(57, snap.TempNowC);
        Assert.True(snap.HasLimits);
    }

    /// <summary>
    /// The case from the report. The same table read with the Zen 3 positions gives nothing
    /// useful; read with the Zen 4 ones it gives the temperature and the power.
    /// </summary>
    [Fact]
    public void Zen4ReadsTemperatureAndPowerFromWhereThatTableKeepsThem()
    {
        var t = Table();
        t[3] = 87;    // CPU PPT
        t[11] = 63;   // package temperature
        t[48] = 41;   // TDC
        t[49] = 96;   // EDC

        var snap = PmTableLayout.Zen4.Read(t, 1f, 5200)!;

        Assert.Equal(63, snap.TempNowC);
        Assert.Equal(87, snap.PptNowW);
        Assert.Equal(41, snap.TdcNowA);
        Assert.Equal(96, snap.EdcNowA);

        // The vendored definitions give no limits for this table, so none are invented.
        Assert.Null(snap.PptLimitW);
        Assert.Null(snap.TempLimitC);
        Assert.False(snap.HasLimits);
    }

    [Fact]
    public void ReadingATableTooShortForTheLayoutYieldsNothingRatherThanThrowing()
    {
        Assert.Null(PmTableLayout.Zen4.Read(new float[20], 1f, 0));
    }

    [Fact]
    public void ThePositionsAreTheOnesTheLayoutClaims()
    {
        Assert.Equal(9, PmTableLayout.Zen3.HighestIndex);
        Assert.Equal(49, PmTableLayout.Zen4.HighestIndex);
    }

    // ── never act on a misread figure ─────────────────────────────

    /// <summary>
    /// A package temperature outside any believable range is a misread table, not a hot
    /// processor. Trusting a low one gives a false sense of protection; reporting nothing is
    /// the honest answer, and the window says so.
    /// </summary>
    [Theory]
    [InlineData(0f)]
    [InlineData(-5f)]
    [InlineData(300f)]
    [InlineData(float.NaN)]
    [InlineData(float.PositiveInfinity)]
    public void ATemperatureOutsideAnyBelievableRangeIsNotReported(float raw)
    {
        var t = Table();
        t[11] = raw;

        Assert.Null(PmTableLayout.Zen4.Read(t, 1f, 0)!.TempNowC);
    }

    [Theory]
    [InlineData(1f)]
    [InlineData(45f)]
    [InlineData(95f)]
    [InlineData(125f)]
    public void ARealTemperatureIsReported(float raw)
    {
        var t = Table();
        t[11] = raw;

        Assert.Equal(raw, PmTableLayout.Zen4.Read(t, 1f, 0)!.TempNowC);
    }

    [Theory]
    [InlineData(0f)]
    [InlineData(-1f)]
    [InlineData(50000f)]
    [InlineData(float.NaN)]
    public void PowerThatIsZeroNegativeOrAbsurdIsNotReported(float raw)
    {
        var t = Table();
        t[3] = raw;

        Assert.Null(PmTableLayout.Zen4.Read(t, 1f, 0)!.PptNowW);
    }

    // ── what the snapshot says about itself ───────────────────────

    [Fact]
    public void ASnapshotWithoutALayoutSaysItsTableIsNotKnown()
    {
        var unknown = new PboSnapshot(1f, 5000, PmTableVersion: 0x620105);

        Assert.False(unknown.TableKnown);
        Assert.Null(unknown.TempNowC);
    }

    [Fact]
    public void ASnapshotReadThroughALayoutSaysItsTableIsKnown()
    {
        var snap = PmTableLayout.Zen3.Read(Table(), 1f, 0)! with { LayoutName = "Zen 3" };

        Assert.True(snap.TableKnown);
    }
}
