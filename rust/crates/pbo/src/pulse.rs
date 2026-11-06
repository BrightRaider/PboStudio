//! Pulses of load: the engine is frozen and released in fractions of a second, hundreds of times a minute, on a clock
//! of its own (the test loop ticks once a second and the pulses are measured in hundreds of milliseconds).
//!
//! The pulser always leaves the engine released: a process left frozen looks exactly like a core that went idle, and
//! the next test would inherit it. Dropping a pulser stops it and waits for it to finish.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Something that can be frozen and released.
pub trait Switch: Send + 'static {
    fn suspend(&self);
    fn resume(&self);
}

impl Switch for pbo_os::Suspender {
    fn suspend(&self) {
        pbo_os::Suspender::suspend(self);
    }
    fn resume(&self) {
        pbo_os::Suspender::resume(self);
    }
}

pub struct Pulser {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

/// Sleeps for `total` in short slices. Returns false when it was asked to stop in the meantime.
fn sleep_unless_stopped(stop: &AtomicBool, total: Duration) -> bool {
    let until = Instant::now() + total;
    loop {
        if stop.load(Ordering::SeqCst) {
            return false;
        }
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return true;
        }
        std::thread::sleep(left.min(Duration::from_millis(10)));
    }
}

impl Pulser {
    /// The load runs for `load`, then the engine is frozen for `idle`, and so on until the pulser is dropped.
    pub fn start<S: Switch>(switch: S, load: Duration, idle: Duration) -> Pulser {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let thread = std::thread::spawn(move || {
            let mut frozen = false;
            loop {
                if !sleep_unless_stopped(&flag, load) {
                    break;
                }
                switch.suspend();
                frozen = true;
                if !sleep_unless_stopped(&flag, idle) {
                    break;
                }
                switch.resume();
                frozen = false;
            }
            if frozen {
                switch.resume();
            }
        });
        Pulser {
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for Pulser {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    #[derive(Clone)]
    struct Counting {
        suspends: Arc<AtomicU32>,
        resumes: Arc<AtomicU32>,
    }

    impl Switch for Counting {
        fn suspend(&self) {
            self.suspends.fetch_add(1, Ordering::SeqCst);
        }
        fn resume(&self) {
            self.resumes.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn counting() -> Counting {
        Counting {
            suspends: Arc::new(AtomicU32::new(0)),
            resumes: Arc::new(AtomicU32::new(0)),
        }
    }

    #[test]
    fn the_load_is_interrupted_again_and_again() {
        let c = counting();
        let p = Pulser::start(
            c.clone(),
            Duration::from_millis(20),
            Duration::from_millis(10),
        );
        std::thread::sleep(Duration::from_millis(300));
        drop(p);
        let n = c.suspends.load(Ordering::SeqCst);
        assert!(n >= 4, "only {n} pulses in 300 ms");
    }

    #[test]
    fn however_it_is_stopped_the_engine_is_left_released() {
        // Stopped at every point of the cycle: during the load and during the freeze.
        for wait_ms in [0, 7, 25, 31, 44, 60, 95] {
            let c = counting();
            let p = Pulser::start(
                c.clone(),
                Duration::from_millis(25),
                Duration::from_millis(40),
            );
            std::thread::sleep(Duration::from_millis(wait_ms));
            drop(p);
            assert_eq!(
                c.suspends.load(Ordering::SeqCst),
                c.resumes.load(Ordering::SeqCst),
                "stopped after {wait_ms} ms"
            );
        }
    }

    // ── A real process ────────────────────────────────────────────────────────
    // The pulses and the pauses freeze and release a real process with the call the engines are frozen with. A busy
    // loop in a PowerShell stands in for an engine: it needs no rights, and it is pinned to one logical processor.

    /// Two of these on one logical processor would share it and each get half: one at a time.
    static ONE_BUSY_PROCESS: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// A process that does nothing but compute, pinned to the last logical processor. `None` where there is no
    /// PowerShell or the process cannot be started: the test is then not run rather than failed.
    fn busy_process() -> Option<pbo_os::Child> {
        let exe =
            std::path::Path::new(r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe");
        if !exe.exists() {
            return None;
        }
        let n = pbo_os::logical_processors();
        if n == 0 || n > 64 {
            return None;
        }
        let child = pbo_os::Child::start_in_job(
            exe,
            "-NoProfile -NonInteractive -Command \"while($true){}\"",
            1usize << (n - 1),
        )
        .ok()?;
        // Let it start up and begin to compute.
        std::thread::sleep(Duration::from_millis(2000));
        Some(child)
    }

    #[test]
    fn bursts_on_a_real_process_run_the_load_about_three_quarters_of_the_time_and_leave_it_released(
    ) {
        let _alone = ONE_BUSY_PROCESS.lock().unwrap_or_else(|e| e.into_inner());
        let Some(child) = busy_process() else {
            return;
        };
        let before = child.cpu_time();
        let began = Instant::now();
        let pulser = Pulser::start(
            child.suspender(),
            Duration::from_millis(300),
            Duration::from_millis(100),
        );
        std::thread::sleep(Duration::from_secs(4));
        drop(pulser);
        let used = (child.cpu_time() - before).as_secs_f64();
        let share = used / began.elapsed().as_secs_f64();
        assert!(
            (0.55..0.92).contains(&share),
            "the process ran {share:.2} of the time, expected about 0.75"
        );
        // Released: it computes again at once.
        let after = child.cpu_time();
        std::thread::sleep(Duration::from_millis(1000));
        let grown = (child.cpu_time() - after).as_secs_f64();
        assert!(
            grown > 0.7,
            "the process only ran {grown:.2} s of the next second"
        );
    }

    #[test]
    fn a_frozen_real_process_uses_no_time_and_a_released_one_computes_again() {
        let _alone = ONE_BUSY_PROCESS.lock().unwrap_or_else(|e| e.into_inner());
        let Some(child) = busy_process() else {
            return;
        };
        child.suspend();
        let a = child.cpu_time();
        std::thread::sleep(Duration::from_millis(1000));
        let frozen = (child.cpu_time() - a).as_secs_f64();
        assert!(
            frozen < 0.1,
            "a frozen process used {frozen:.2} s in a second"
        );
        child.resume();
        let b = child.cpu_time();
        std::thread::sleep(Duration::from_millis(1000));
        let running = (child.cpu_time() - b).as_secs_f64();
        assert!(
            running > 0.7,
            "a released process used only {running:.2} s in a second"
        );
    }

    #[test]
    fn dropping_waits_for_the_thread_so_nothing_runs_after_it() {
        let c = counting();
        let p = Pulser::start(
            c.clone(),
            Duration::from_millis(5),
            Duration::from_millis(5),
        );
        std::thread::sleep(Duration::from_millis(40));
        drop(p);
        let (s, r) = (
            c.suspends.load(Ordering::SeqCst),
            c.resumes.load(Ordering::SeqCst),
        );
        std::thread::sleep(Duration::from_millis(60));
        assert_eq!(
            (s, r),
            (
                c.suspends.load(Ordering::SeqCst),
                c.resumes.load(Ordering::SeqCst)
            )
        );
    }
}
