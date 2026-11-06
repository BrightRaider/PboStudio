//! Hours and minutes that keep counting past a day (invariant I10).
//!
//! The built-in `h:mm` style prints the hours *component*, 0 to 23, so one day came out as `0:00` and the
//! elapsed counter of a long run jumped back to zero overnight. Vectors: `spec/vectors/duration-format.json`.

/// `26:05` for a day, two hours and five minutes. A negative span shows as zero. Minutes are truncated, not
/// rounded: 59 minutes 59 seconds is `0:59`.
pub fn hours(seconds: i64) -> String {
    let s = seconds.max(0);
    format!("{}:{:02}", s / 3600, s % 3600 / 60)
}

/// `26:05:09`.
pub fn hours_seconds(seconds: i64) -> String {
    let s = seconds.max(0);
    format!("{}:{:02}:{:02}", s / 3600, s % 3600 / 60, s % 60)
}
