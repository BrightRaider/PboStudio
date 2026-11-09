//! `pbo report`: one file to attach to an issue or to read back later.
//!
//! What anyone who helps with a problem asks for, gathered into `pbo-report.txt` next to the program: the version, what the
//! processor holds, what the program remembers about each core, the machine checks Windows has logged, how Auto was
//! started, the end of the newest log, and the file `pbo diag` wrote if there is one. It runs the program itself for the
//! parts that have a command of their own, so the report says what those commands say.

use std::path::Path;

fn run_self(exe: &Path, args: &[&str]) -> String {
    match std::process::Command::new(exe).args(args).output() {
        Ok(out) => {
            let mut text = String::from_utf8_lossy(&out.stdout).to_string();
            let err = String::from_utf8_lossy(&out.stderr);
            if !err.trim().is_empty() {
                text.push_str(&err);
            }
            text
        }
        Err(e) => format!("(could not run: {e})\n"),
    }
}

fn section(report: &mut String, title: &str, body: &str) {
    report.push_str(&format!("\n===== {title} =====\n"));
    let body = body.trim_end();
    report.push_str(if body.is_empty() { "(nothing)" } else { body });
    report.push('\n');
}

/// The last `lines` lines of the newest daily log, without the readings (they are every few seconds and drown the rest).
fn log_tail(logs: &Path, lines: usize) -> String {
    let mut files: Vec<_> = std::fs::read_dir(logs)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("pbo_"))
        })
        .collect();
    files.sort();
    let Some(newest) = files.last() else {
        return String::new();
    };
    let text = std::fs::read_to_string(newest).unwrap_or_default();
    let kept: Vec<&str> = text
        .lines()
        .filter(|l| !l.contains(" telemetry "))
        .collect();
    let skip = kept.len().saturating_sub(lines);
    format!(
        "{}\n{}",
        newest.file_name().unwrap_or_default().to_string_lossy(),
        kept[skip..].join("\n")
    )
}

pub fn run(_args: &[String]) -> i32 {
    let Ok(exe) = std::env::current_exe() else {
        eprintln!("cannot find this program");
        return 1;
    };
    let dir = crate::store::program_dir();
    let runs = dir.join("runs");
    let mut report = String::from(
        "PboStudio report. Look through it before you attach it: it names your processor, BIOS version and what the program logged.\n",
    );
    section(&mut report, "version", &run_self(&exe, &["version"]));
    section(&mut report, "status", &run_self(&exe, &["status"]));
    section(
        &mut report,
        "what the program remembers",
        &run_self(&exe, &["knowledge"]),
    );
    section(
        &mut report,
        "machine checks Windows logged, 90 days",
        &run_self(&exe, &["whea", "--days", "90"]),
    );
    for name in ["auto_last.txt", "auto_resume.txt", "window.json"] {
        if let Ok(text) = std::fs::read_to_string(runs.join(name)) {
            section(&mut report, name, &text);
        }
    }
    section(
        &mut report,
        "the newest log (readings left out)",
        &log_tail(&runs.join("logs"), 300),
    );
    if let Ok(diag) = std::fs::read_to_string(dir.join("pbo-diag.txt")) {
        let head: Vec<&str> = diag.lines().take(250).collect();
        section(
            &mut report,
            "pbo-diag.txt (the first 250 lines)",
            &head.join("\n"),
        );
    }
    let path = dir.join("pbo-report.txt");
    match std::fs::write(&path, report) {
        Ok(()) => {
            println!("Written: {}", path.display());
            0
        }
        Err(e) => {
            eprintln!("could not write {}: {e}", path.display());
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tail_of_the_newest_log_leaves_the_readings_out() {
        let dir = std::env::temp_dir().join(format!("pbo-report-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("pbo_2026-11-05.log"), "old day\n").unwrap();
        std::fs::write(
            dir.join("pbo_2026-11-06.log"),
            "first\n12:00:00 [1] telemetry core 0: temp 60.0C\nsecond\n12:00:05 [1] telemetry core 0: temp 61.0C\nthird\n",
        )
        .unwrap();
        let tail = log_tail(&dir, 2);
        assert!(tail.starts_with("pbo_2026-11-06.log"), "{tail}");
        assert!(tail.contains("second") && tail.contains("third"), "{tail}");
        assert!(
            !tail.contains("first") && !tail.contains("telemetry"),
            "{tail}"
        );
        // No logs at all is an empty section, not an error.
        let empty = std::env::temp_dir().join(format!("pbo-report-none-{}", std::process::id()));
        assert_eq!(log_tail(&empty, 10), "");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_section_with_nothing_in_it_says_so() {
        let mut report = String::new();
        section(&mut report, "knowledge", "  \n");
        assert!(report.contains("===== knowledge ====="));
        assert!(report.contains("(nothing)"));
    }
}
