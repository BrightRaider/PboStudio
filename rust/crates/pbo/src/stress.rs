//! Starting a stress engine on one core and watching it (invariants I2, I4, I5).
//!
//! The engine runs as a child inside a job object whose affinity is the core's logical processors, from its first
//! instruction: a process can widen its own affinity but cannot leave the job's. Failures are read from the engine's
//! output file, whole lines only, once per poll.

use std::fs;
use std::path::{Path, PathBuf};

use pbo_core::engine::{self, FftPreset, Line, Prime95Cycles, Prime95Mode};
use pbo_core::logtail::LogTail;
use pbo_os::{Child, Priority};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineKind {
    Prime95,
    YCruncher,
}

impl EngineKind {
    pub fn name(self) -> &'static str {
        match self {
            EngineKind::Prime95 => "Prime95",
            EngineKind::YCruncher => "y-cruncher",
        }
    }

    pub fn from_name(name: &str) -> Option<EngineKind> {
        match name.to_ascii_lowercase().as_str() {
            "prime95" | "p95" => Some(EngineKind::Prime95),
            "ycruncher" | "y-cruncher" | "yc" => Some(EngineKind::YCruncher),
            _ => None,
        }
    }
}

/// Where the engines are, if they are installed: `engines/` next to the program, or in a source checkout the
/// `engines/` or `external/engines/` of a parent folder.
pub fn engine_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(custom) = std::env::var("PBO_ENGINES") {
        roots.push(PathBuf::from(custom));
    }
    let mut dir = Some(crate::store::program_dir());
    for _ in 0..6 {
        let Some(d) = dir else { break };
        roots.push(d.join("engines"));
        roots.push(d.join("external").join("engines"));
        dir = d.parent().map(Path::to_path_buf);
    }
    roots
}

pub fn find_prime95() -> Option<PathBuf> {
    engine_roots()
        .into_iter()
        .map(|r| r.join("prime95").join("prime95.exe"))
        .find(|p| p.is_file())
}

/// The y-cruncher binary for a Zen generation (1 to 5): the first file whose name starts with the preferred prefix.
pub fn find_ycruncher(generation: u32) -> Option<PathBuf> {
    for root in engine_roots() {
        let base = root.join("ycruncher");
        let Ok(installs) = fs::read_dir(&base) else {
            continue;
        };
        for install in installs.filter_map(Result::ok) {
            let binaries = install.path().join("Binaries");
            let Ok(files) = fs::read_dir(&binaries) else {
                continue;
            };
            let names: Vec<PathBuf> = files
                .filter_map(Result::ok)
                .map(|f| f.path())
                .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe")))
                .collect();
            for prefix in engine::ycruncher_preferred_prefixes(generation) {
                if let Some(hit) = names.iter().find(|p| {
                    p.file_name()
                        .is_some_and(|n| n.to_string_lossy().starts_with(prefix))
                }) {
                    return Some(hit.clone());
                }
            }
            if let Some(first) = names.first() {
                return Some(first.clone());
            }
        }
    }
    None
}

#[derive(Debug, Clone)]
pub struct Setup {
    pub kind: EngineKind,
    pub exe: PathBuf,
    pub fft: FftPreset,
    pub mode: Prime95Mode,
    /// y-cruncher's seconds per algorithm.
    pub seconds_per_test: u32,
    /// Which algorithms y-cruncher runs.
    pub yc_set: engine::YcSet,
    /// y-cruncher's tests of the person's own choosing (their tags); none: the set above.
    pub yc_tests: Option<Vec<String>>,
    /// The memory y-cruncher gets for each logical processor, in MiB; none: 64.
    pub yc_memory_mb: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub text: String,
    /// The engine ended on its own, which neither does during a stress test.
    pub process_gone: bool,
}

pub struct Stress {
    child: Child,
    kind: EngineKind,
    output: PathBuf,
    tail: LogTail,
    cycles: Prime95Cycles,
    yc_headers: u32,
    exit_reported: bool,
}

/// The bits of `threads` logical processors, as an affinity mask. `None` when one does not fit in a single
/// processor group (more than 64 logical processors are not supported yet).
pub fn affinity_mask(logical: &[usize]) -> Option<usize> {
    let mut mask = 0usize;
    for l in logical {
        if *l >= usize::BITS as usize {
            return None;
        }
        mask |= 1 << l;
    }
    Some(mask)
}

impl Stress {
    pub fn start(
        setup: &Setup,
        work_root: &Path,
        core: usize,
        logical: &[usize],
    ) -> Result<Stress, String> {
        let mask = affinity_mask(logical).ok_or(
            "this processor has more than 64 logical processors, which are not supported yet",
        )?;
        let threads = logical.len() as u32;
        match setup.kind {
            EngineKind::Prime95 => {
                // A directory per core keeps each run's results separate, so a failure is attributed without
                // parsing which worker wrote which line.
                let dir = work_root.join(format!("core{core}"));
                if dir.exists() {
                    for f in fs::read_dir(&dir)
                        .into_iter()
                        .flatten()
                        .filter_map(Result::ok)
                    {
                        let _ = fs::remove_file(f.path());
                    }
                }
                fs::create_dir_all(&dir).map_err(|e| format!("work folder: {e}"))?;
                let config = engine::prime95_config(setup.mode, setup.fft, 0, threads);
                fs::write(dir.join("prime.txt"), config.as_bytes())
                    .map_err(|e| format!("prime.txt: {e}"))?;
                // Prime95 takes `-W` literally: a quotation mark in it is part of the folder name, the folder is then not
                // found, and it falls back to the settings in its own folder (which is how it used every core of the
                // machine and reported nothing). So the path goes in without quotes; a path with a space has to be given
                // by its short name. Found by comparing with CoreCycler, which does not use -W at all.
                let folder = pbo_os::short_path(&dir).unwrap_or_else(|| dir.display().to_string());
                if folder.contains(' ') {
                    return Err(format!(
                        "the work folder '{}' has a space in its path and no short name, and Prime95 cannot be pointed at it",
                        dir.display()
                    ));
                }
                let args = format!("-W{folder} -t");
                let child = Child::start(&setup.exe, &args, mask, Some(&dir), Priority::Normal)?;
                Ok(Stress {
                    child,
                    kind: setup.kind,
                    output: dir.join("results.txt"),
                    tail: LogTail::default(),
                    cycles: Prime95Cycles::default(),
                    yc_headers: 0,
                    exit_reported: false,
                })
            }
            EngineKind::YCruncher => {
                let dir = work_root.join(format!("yc-core{core}"));
                fs::create_dir_all(&dir).map_err(|e| format!("work folder: {e}"))?;
                let log = dir.join(format!("output-{}.txt", std::process::id()));
                // The settings go in a file, because only there can y-cruncher be told which logical processors to run
                // on; without it, it starts a thread for each of the machine's and they take turns on the one it may use.
                let config = dir.join(format!("stress-{}.cfg", std::process::id()));
                // The person's own list of tests and memory, or the set that was chosen and 64 MiB for each thread.
                let custom: Vec<&str> = setup
                    .yc_tests
                    .as_ref()
                    .map(|t| t.iter().map(String::as_str).collect())
                    .unwrap_or_default();
                let algorithms: &[&str] = if custom.is_empty() {
                    setup.yc_set.algorithms()
                } else {
                    &custom
                };
                let memory = setup
                    .yc_memory_mb
                    .map_or(engine::YCRUNCHER_MEMORY_PER_THREAD, |m| u64::from(m) << 20);
                fs::write(
                    &config,
                    engine::ycruncher_config_with(
                        logical,
                        setup.seconds_per_test,
                        algorithms,
                        memory,
                    )
                    .as_bytes(),
                )
                .map_err(|e| format!("y-cruncher settings file: {e}"))?;
                let args = engine::ycruncher_arguments(
                    &log.display().to_string(),
                    &config.display().to_string(),
                );
                let work = setup.exe.parent().map(Path::to_path_buf);
                let child =
                    Child::start(&setup.exe, &args, mask, work.as_deref(), Priority::Normal)?;
                Ok(Stress {
                    child,
                    kind: setup.kind,
                    output: log,
                    tail: LogTail::default(),
                    cycles: Prime95Cycles::default(),
                    yc_headers: 0,
                    exit_reported: false,
                })
            }
        }
    }

    pub fn is_alive(&self) -> bool {
        self.child.is_alive()
    }

    /// Finished sweeps of the engine's work list.
    pub fn cycles(&self) -> u32 {
        match self.kind {
            EngineKind::Prime95 => self.cycles.completed,
            EngineKind::YCruncher => engine::ycruncher_cycles(self.yc_headers),
        }
    }

    pub fn cpu_time(&self) -> std::time::Duration {
        self.child.cpu_time()
    }

    pub fn stop(&self) {
        self.child.kill();
    }

    /// Freezes the engine and releases it again: the pauses of a test that interrupts its load.
    pub fn suspend(&self) {
        self.child.suspend();
    }

    pub fn resume(&self) {
        self.child.resume();
    }

    /// The handle for a pulser on a thread of its own. It must not outlive the stress.
    pub fn suspender(&self) -> pbo_os::Suspender {
        self.child.suspender()
    }

    /// New failures since the last poll: from the new complete lines of the output file, and, once, the engine
    /// having ended by itself.
    pub fn poll(&mut self) -> Vec<Failure> {
        let mut failures = Vec::new();
        let content = fs::read(&self.output).ok();
        for line in self.tail.poll(content.as_deref()) {
            match self.kind {
                EngineKind::Prime95 => {
                    let classified = engine::classify_prime95(&line);
                    self.cycles.feed(&classified);
                    if let Line::Failure(text) = classified {
                        failures.push(Failure {
                            text,
                            process_gone: false,
                        });
                    }
                }
                EngineKind::YCruncher => {
                    if engine::is_ycruncher_iteration_header(&line) {
                        self.yc_headers += 1;
                    }
                    if let Line::Failure(text) = engine::classify_ycruncher(&line) {
                        failures.push(Failure {
                            text,
                            process_gone: false,
                        });
                    }
                }
            }
        }
        if !self.is_alive() && !self.exit_reported {
            self.exit_reported = true;
            failures.push(Failure {
                text: format!(
                    "{} ended on its own, which it does not do during a stress test (exit code {})",
                    self.kind.name(),
                    self.child
                        .exit_code()
                        .map_or("?".to_string(), |c| c.to_string())
                ),
                process_gone: true,
            });
        }
        failures
    }
}

#[cfg(test)]
mod real_engine {
    use super::*;
    use std::time::{Duration, Instant};

    /// The whole way a failure takes, with the real engine and without a real error: a line Prime95 writes for a failed
    /// check is appended to its results file while it runs, and the stress has to report it.
    #[test]
    #[ignore]
    fn a_failure_line_in_the_results_file_of_a_running_prime95_is_found() {
        use std::io::Write;
        let Some(exe) = find_prime95() else {
            eprintln!("no Prime95 found: not run");
            return;
        };
        let logical = pbo_os::logical_processors() - 1;
        let work_root =
            std::env::temp_dir().join(format!("pbo-real-failure-{}", std::process::id()));
        let setup = Setup {
            kind: EngineKind::Prime95,
            exe,
            fft: FftPreset::Custom(4, 5),
            mode: Prime95Mode::Sse,
            seconds_per_test: 60,
            yc_set: engine::YcSet::default(),
            yc_tests: None,
            yc_memory_mb: None,
        };
        let mut stress = Stress::start(&setup, &work_root, 7, &[logical]).expect("start");
        let results = work_root.join("core7").join("results.txt");
        let began = Instant::now();
        // While the engine runs normally nothing is a failure.
        while !results.exists() && began.elapsed() < Duration::from_secs(60) {
            std::thread::sleep(Duration::from_secs(1));
            assert!(stress.poll().is_empty(), "a failure before there was one");
        }
        assert!(results.exists(), "Prime95 wrote no results in 60 s");
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&results)
            .expect("open");
        writeln!(
            file,
            "[Worker #1 Nov 6 10:00] Hardware failure detected running 8K FFT size, consult stress.txt file."
        )
        .expect("write");
        drop(file);
        let mut found = Vec::new();
        for _ in 0..10 {
            std::thread::sleep(Duration::from_secs(1));
            found = stress.poll();
            if !found.is_empty() {
                break;
            }
        }
        stress.stop();
        drop(stress);
        let _ = fs::remove_dir_all(&work_root);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].text.contains("Hardware failure detected"),
            "{found:?}"
        );
    }

    /// y-cruncher on one logical processor has to run one thread and say so (its log header names the logical cores it
    /// uses), report a finished iteration, and report no failure. Needs the engine next to the program.
    #[test]
    #[ignore]
    fn ycruncher_on_one_processor_runs_one_thread() {
        let Some(exe) = find_ycruncher(3) else {
            eprintln!("no y-cruncher found: not run");
            return;
        };
        let logical = pbo_os::logical_processors() - 1;
        let work_root = std::env::temp_dir().join(format!("pbo-real-yc-{}", std::process::id()));
        let setup = Setup {
            kind: EngineKind::YCruncher,
            exe,
            fft: FftPreset::Custom(4, 5),
            mode: Prime95Mode::Sse,
            seconds_per_test: 3,
            yc_set: engine::YcSet::FastDiscovery,
            yc_tests: None,
            yc_memory_mb: None,
        };
        let mut stress = Stress::start(&setup, &work_root, 7, &[logical]).expect("start");
        let log = work_root
            .join("yc-core7")
            .join(format!("output-{}.txt", std::process::id()));
        let began = Instant::now();
        let mut failures = Vec::new();
        while began.elapsed() < Duration::from_secs(40) && stress.cycles() < 1 {
            std::thread::sleep(Duration::from_secs(1));
            failures.extend(stress.poll());
        }
        let text = fs::read_to_string(&log).unwrap_or_default();
        let cycles = stress.cycles();
        stress.stop();
        drop(stress);
        let _ = fs::remove_dir_all(&work_root);
        assert!(failures.is_empty(), "{failures:?}\n{text}");
        assert!(text.contains("Logical Cores:      1"), "{text}");
        assert!(!text.contains("Failed to set core affinity"), "{text}");
        assert!(cycles >= 1, "no iteration finished in 40 s:\n{text}");
    }

    #[test]
    #[ignore]
    fn prime95_confined_to_one_processor_reports_a_finished_fft_size() {
        let Some(exe) = find_prime95() else {
            eprintln!("no Prime95 found: not run");
            return;
        };
        let n = pbo_os::logical_processors();
        let logical = n - 1;
        // PBO_TEST_ALL=1 lets the engine use every processor, to tell the confinement from the rest of the launch.
        let logical_list: Vec<usize> = if std::env::var("PBO_TEST_ALL").is_ok() {
            (0..n.min(64)).collect()
        } else {
            vec![logical]
        };
        let work_root =
            std::env::temp_dir().join(format!("pbo-real-engine-{}", std::process::id()));
        let setup = Setup {
            kind: EngineKind::Prime95,
            exe,
            fft: FftPreset::Custom(4, 5),
            mode: Prime95Mode::Sse,
            seconds_per_test: 60,
            yc_set: engine::YcSet::default(),
            yc_tests: None,
            yc_memory_mb: None,
        };
        let mut stress = Stress::start(&setup, &work_root, 7, &logical_list).expect("start");
        let began = Instant::now();
        let mut reported = None;
        while began.elapsed() < Duration::from_secs(60) {
            std::thread::sleep(Duration::from_secs(2));
            let _ = stress.poll();
            let results = work_root.join("core7").join("results.txt");
            if let Ok(text) = fs::read_to_string(&results) {
                if !text.trim().is_empty() {
                    reported = Some((began.elapsed(), text));
                    break;
                }
            }
        }
        let files: Vec<String> = fs::read_dir(work_root.join("core7"))
            .map(|d| {
                d.filter_map(Result::ok)
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default();
        eprintln!("files in the work folder: {files:?}");
        eprintln!(
            "alive: {}, cpu time used: {:?}",
            stress.is_alive(),
            stress.cpu_time()
        );
        if let Ok(text) = fs::read_to_string(work_root.join("core7").join("prime.txt")) {
            eprintln!(
                "prime.txt:
{text}"
            );
        }
        stress.stop();
        drop(stress);
        let _ = fs::remove_dir_all(&work_root);
        let (after, text) = reported.expect("Prime95 reported nothing in 60 s");
        eprintln!(
            "first report after {after:?}: {}",
            text.lines().last().unwrap_or("")
        );
    }
}
