using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

/// <summary>
/// "h:mm" on a TimeSpan prints the hours component, 0 to 23. One day came out as "0:00", so the
/// elapsed counter of a long run jumped back to zero overnight.
/// </summary>
public class DurationFormatTests
{
    [Theory]
    [InlineData(0, 0, 0, "0:00")]
    [InlineData(0, 5, 0, "0:05")]
    [InlineData(2, 24, 0, "2:24")]
    [InlineData(23, 59, 0, "23:59")]
    [InlineData(24, 0, 0, "24:00")]
    [InlineData(26, 5, 0, "26:05")]
    [InlineData(100, 30, 0, "100:30")]
    public void HoursKeepCountingPastADay(int hours, int minutes, int seconds, string expected)
    {
        Assert.Equal(expected, DurationFormat.Hours(new TimeSpan(hours, minutes, seconds)));
    }

    [Fact]
    public void TheBuiltInFormatIsTheBugThisReplaces()
    {
        // Documented, so nobody "simplifies" it back: one day through the custom format.
        Assert.Equal("0:00", TimeSpan.FromDays(1).ToString(@"h\:mm"));
        Assert.Equal("24:00", DurationFormat.Hours(TimeSpan.FromDays(1)));
    }

    [Theory]
    [InlineData(0, 0, 9, "0:00:09")]
    [InlineData(1, 2, 3, "1:02:03")]
    [InlineData(25, 0, 1, "25:00:01")]
    public void HoursWithSecondsKeepCountingToo(int h, int m, int s, string expected)
    {
        Assert.Equal(expected, DurationFormat.HoursSeconds(new TimeSpan(h, m, s)));
    }

    [Fact]
    public void ANegativeSpanIsShownAsZeroRatherThanAMinusSign()
    {
        Assert.Equal("0:00", DurationFormat.Hours(TimeSpan.FromMinutes(-5)));
        Assert.Equal("0:00:00", DurationFormat.HoursSeconds(TimeSpan.FromSeconds(-1)));
    }

    /// <summary>
    /// A whole minute never rounds up into a wrong hour: 59 minutes 59 seconds is 0:59, not 1:00.
    /// </summary>
    [Fact]
    public void MinutesAreTruncatedNotRounded()
    {
        Assert.Equal("0:59", DurationFormat.Hours(new TimeSpan(0, 59, 59)));
    }
}
