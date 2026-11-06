//! Starting Windows' own tools safely: by full path in the system folder, with a timeout that holds.
//!
//! This program runs as administrator. A tool started by bare name is looked for in the program's own folder first, so
//! whatever sits next to the program with the right name would run with those rights; the system folder is not writable
//! by an ordinary user. And a tool that hangs must not freeze the thread that asked: both output streams are read in the
//! background and the wait is the only thing the caller blocks on, with a limit.

use std::io::Read;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// What a finished tool said.
#[derive(Debug, Clone)]
pub struct Output {
    pub success: bool,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl Output {
    pub fn stdout_text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }

    pub fn stderr_text(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }
}

/// `C:\Windows\System32\<relative>` when it exists. `None` when it does not: a missing tool is reported as missing, not
/// replaced by whatever the search path would find.
pub fn system_tool(relative_to_system32: &str) -> Option<PathBuf> {
    let root = std::env::var_os("SystemRoot")?;
    let full = PathBuf::from(root)
        .join("System32")
        .join(relative_to_system32);
    full.is_file().then_some(full)
}

/// Runs a system tool to completion. `None` when the tool is missing, cannot be started, or does not finish in `timeout`
/// (it is ended then).
pub fn run(relative_to_system32: &str, args: &[&str], timeout: Duration) -> Option<Output> {
    let path = system_tool(relative_to_system32)?;
    let mut child = Command::new(path)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .ok()?;

    fn drain<R: Read + Send + 'static>(mut r: R) -> std::thread::JoinHandle<Vec<u8>> {
        std::thread::spawn(move || {
            let mut buffer = Vec::new();
            let _ = r.read_to_end(&mut buffer);
            buffer
        })
    }
    let out = drain(child.stdout.take()?);
    let err = drain(child.stderr.take()?);

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    // Once the process is gone its pipes close and the readers end.
    let stdout = out.join().unwrap_or_default();
    let stderr = err.join().unwrap_or_default();
    status.map(|s| Output {
        success: s.success(),
        stdout,
        stderr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_system_tool_is_found_by_full_path_and_run() {
        let o = run("cmd.exe", &["/c", "echo hello"], Duration::from_secs(10)).unwrap();
        assert!(o.success);
        assert!(o.stdout_text().contains("hello"));
    }

    #[test]
    fn a_tool_that_hangs_is_ended_at_the_timeout() {
        let start = Instant::now();
        let o = run(
            "ping.exe",
            &["-n", "30", "127.0.0.1"],
            Duration::from_millis(600),
        );
        assert!(
            o.is_none(),
            "a tool that did not finish must not look like a result"
        );
        assert!(start.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn a_tool_that_is_not_there_is_reported_missing() {
        assert!(system_tool("definitely-not-a-windows-tool.exe").is_none());
        assert!(run(
            "definitely-not-a-windows-tool.exe",
            &[],
            Duration::from_secs(1)
        )
        .is_none());
    }

    #[test]
    fn a_failing_tool_says_so() {
        let o = run("cmd.exe", &["/c", "exit 3"], Duration::from_secs(10)).unwrap();
        assert!(!o.success);
    }
}
