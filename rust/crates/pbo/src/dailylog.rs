//! The log: one file per day, the night belongs to the first day, nothing is deleted (ADR-0008, point 7).
//!
//! A run keeps writing into the file it began in until it ends, however long that takes. Every line is written to
//! the file as it happens, so it survives a crash. Several starts on one day, also after crashes, land in the same
//! file and are separated by a start line.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use pbo_core::logday::{self, Date};
use pbo_os as os;

/// A day's file grows past this and the next part begins; nothing is cut off.
const PART_LIMIT_BYTES: u64 = 25 * 1024 * 1024;
/// The log day ends at this local hour, not at midnight.
pub const DEFAULT_BOUNDARY_HOUR: u32 = 6;

pub struct DailyLog {
    file: File,
    pub path: PathBuf,
    run_id: String,
}

/// A short identifier for a run, from the clock: enough to filter one run out of several days of lines.
pub fn new_run_id() -> String {
    let t = os::local_time();
    format!(
        "{:02}{:02}{:02}{:02}{:02}",
        t.month, t.day, t.hour, t.minute, t.second
    )
}

impl DailyLog {
    /// Opens the file for the log day of this moment and writes the start line. `header` is written first when the
    /// file is new, so that each file can be read by itself.
    pub fn open(
        logs_dir: PathBuf,
        run_id: &str,
        boundary_hour: u32,
        header: &[String],
    ) -> std::io::Result<DailyLog> {
        fs::create_dir_all(&logs_dir)?;
        let now = os::local_time();
        let today = Date {
            year: now.year,
            month: now.month,
            day: now.day,
        };
        let day = logday::log_day(today, now.hour, boundary_hour);

        // The first part of the day that still has room.
        let mut sizes = Vec::new();
        for part in 1.. {
            let path = logs_dir.join(logday::file_name(day, part));
            match fs::metadata(&path) {
                Ok(m) => sizes.push(m.len()),
                Err(_) => break,
            }
        }
        let part = logday::part_for(&sizes, PART_LIMIT_BYTES);
        let path = logs_dir.join(logday::file_name(day, part));
        let is_new = !path.exists();
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        let mut log = DailyLog {
            file,
            path,
            run_id: run_id.to_string(),
        };
        if is_new {
            for h in header {
                log.raw(h)?;
            }
            log.raw("")?;
        }
        log.line("---- start ----")?;
        Ok(log)
    }

    fn raw(&mut self, text: &str) -> std::io::Result<()> {
        writeln!(self.file, "{text}")?;
        self.file.flush()
    }

    /// One line with the time and the run id.
    pub fn line(&mut self, text: &str) -> std::io::Result<()> {
        let t = os::local_time();
        let line = logday::line(t.hour, t.minute, t.second, &self.run_id, text);
        self.raw(&line)
    }

    /// A line that must survive a power cut: pushed to the disk, not only to the system's cache.
    pub fn important(&mut self, text: &str) -> std::io::Result<()> {
        self.line(text)?;
        self.file.sync_data()
    }
}
