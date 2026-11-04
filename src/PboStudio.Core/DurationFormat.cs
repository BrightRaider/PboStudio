namespace PboStudio.Core;

/// <summary>
/// Hours and minutes that keep counting past a day.
///
/// The custom TimeSpan format "h:mm" prints the hours <em>component</em>, which is 0 to 23: a
/// duration of exactly one day came out as "0:00". Runs here routinely pass 24 hours - the
/// confirmation phase alone is sixteen on an eight-core part - so the elapsed counter jumped back
/// to zero in the middle of the night, and an estimate of a day and a half read as twelve hours.
/// </summary>
public static class DurationFormat
{
    /// <summary>"26:05" for a day, two hours and five minutes.</summary>
    public static string Hours(TimeSpan span)
    {
        if (span < TimeSpan.Zero) span = TimeSpan.Zero;
        return $"{(long)span.TotalHours}:{span.Minutes:D2}";
    }

    /// <summary>"26:05:09".</summary>
    public static string HoursSeconds(TimeSpan span)
    {
        if (span < TimeSpan.Zero) span = TimeSpan.Zero;
        return $"{(long)span.TotalHours}:{span.Minutes:D2}:{span.Seconds:D2}";
    }
}
