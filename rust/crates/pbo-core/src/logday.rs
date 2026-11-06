//! Which day's log a moment belongs to, and what the file is called (ADR-0008, point 7).
//!
//! One log per day, not one per start and not an endless one. A night belongs to the day it began: the log day ends
//! at 06:00, not at midnight, so a line written at 03:00 on the 6th goes into the file of the 5th. A run that is
//! already going keeps its file until it ends, even across that boundary; that part is the caller's, this module
//! only answers "which day does this moment belong to" for the moment a run starts or a new line is written without
//! a run. Vectors: `spec/vectors/log-day.json`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

pub fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => 0,
    }
}

impl Date {
    pub fn previous(self) -> Date {
        if self.day > 1 {
            Date {
                day: self.day - 1,
                ..self
            }
        } else if self.month > 1 {
            Date {
                year: self.year,
                month: self.month - 1,
                day: days_in_month(self.year, self.month - 1),
            }
        } else {
            Date {
                year: self.year - 1,
                month: 12,
                day: 31,
            }
        }
    }
}

/// The log day a moment belongs to: the calendar day, or the day before when the local hour is earlier than
/// `boundary_hour` (06 by default).
pub fn log_day(date: Date, hour: u32, boundary_hour: u32) -> Date {
    if hour < boundary_hour {
        date.previous()
    } else {
        date
    }
}

/// `pbo_2026-11-05.log`, or `pbo_2026-11-05_2.log` for the second file of a day that grew very large.
pub fn file_name(day: Date, part: u32) -> String {
    if part <= 1 {
        format!("pbo_{:04}-{:02}-{:02}.log", day.year, day.month, day.day)
    } else {
        format!(
            "pbo_{:04}-{:02}-{:02}_{part}.log",
            day.year, day.month, day.day
        )
    }
}

/// One line of the log: the time, the run it belongs to, the text. The run id lets a pass over several days be
/// filtered out of the files.
pub fn line(hour: u32, minute: u32, second: u32, run_id: &str, text: &str) -> String {
    format!("{hour:02}:{minute:02}:{second:02} [{run_id}] {text}")
}

/// The part to write to: the first whose size is still under `limit_bytes`. `sizes[0]` is part 1.
pub fn part_for(sizes: &[u64], limit_bytes: u64) -> u32 {
    for (i, size) in sizes.iter().enumerate() {
        if *size < limit_bytes {
            return i as u32 + 1;
        }
    }
    sizes.len() as u32 + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(year: i32, month: u32, day: u32) -> Date {
        Date { year, month, day }
    }

    #[test]
    fn the_night_belongs_to_the_evening() {
        assert_eq!(log_day(d(2026, 11, 6), 3, 6), d(2026, 11, 5));
        assert_eq!(log_day(d(2026, 11, 6), 5, 6), d(2026, 11, 5));
    }

    #[test]
    fn six_in_the_morning_starts_the_new_day() {
        assert_eq!(log_day(d(2026, 11, 6), 6, 6), d(2026, 11, 6));
        assert_eq!(log_day(d(2026, 11, 6), 23, 6), d(2026, 11, 6));
    }

    #[test]
    fn the_day_before_crosses_months_and_years() {
        assert_eq!(d(2026, 3, 1).previous(), d(2026, 2, 28));
        assert_eq!(d(2028, 3, 1).previous(), d(2028, 2, 29));
        assert_eq!(d(2026, 1, 1).previous(), d(2025, 12, 31));
        assert_eq!(d(2026, 5, 1).previous(), d(2026, 4, 30));
    }

    #[test]
    fn a_boundary_of_zero_is_the_calendar_day() {
        assert_eq!(log_day(d(2026, 11, 6), 0, 0), d(2026, 11, 6));
    }

    #[test]
    fn names_and_parts() {
        assert_eq!(file_name(d(2026, 11, 5), 1), "pbo_2026-11-05.log");
        assert_eq!(file_name(d(2026, 11, 5), 2), "pbo_2026-11-05_2.log");
        assert_eq!(part_for(&[], 100), 1);
        assert_eq!(part_for(&[100], 100), 2);
        assert_eq!(part_for(&[100, 50], 100), 2);
    }

    #[test]
    fn a_line_carries_the_run() {
        assert_eq!(
            line(3, 4, 5, "a1b2", "core 0 ok"),
            "03:04:05 [a1b2] core 0 ok"
        );
    }
}
