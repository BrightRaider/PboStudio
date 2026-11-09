//! One test of one core at one value, with everything that has to be watched while it runs (invariants I2 to I9).
//!
//! The engine runs pinned to the core. Once a second its output is read for failures; every five seconds a line of
//! readings goes into the log (temperature, power, core voltage); every ten seconds the event log is asked for
//! machine checks, which are attributed to a core by the processor's own APIC ids. The state file names the value
//! under test, so a crash can be told from a closed window afterwards.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use pbo_core::knowledge::Knowledge;
use pbo_core::whea::{self, WheaEvent};
use pbo_os as os;

use crate::dailylog::DailyLog;
use crate::eventlog;
use crate::machine::Machine;
use crate::store::{RunState, Store};
use crate::stress::{self, EngineKind, Failure, Setup, Stress};

/// What a running session tells whoever is watching it (the window). The command line watches the printed lines.
#[derive(Debug, Clone)]
pub enum Event {
    Line(String),
    Sample(crate::machine::Sample),
    /// A core is now under test at this value.
    Progress {
        core: usize,
        margin: i32,
        seconds: u64,
    },
    Finished(i32),
}

pub struct Session<'a> {
    pub machine: &'a Machine,
    pub store: &'a Store,
    pub log: DailyLog,
    pub setup: Setup,
    /// Further steps per core after the first: a core passes a value only when every step passes.
    pub extra_legs: Vec<Setup>,
    pub run_id: String,
    pub started_utc: String,
    pub baseline: Vec<i32>,
    pub knowledge: Knowledge,
    pub bios: String,
    /// Test both threads of a core instead of one.
    pub smt: bool,
    /// How the load is interrupted so the core has to boost back up.
    pub transient: pbo_core::transient::Transient,
    pub events: Option<std::sync::mpsc::Sender<Event>>,
    /// Holds the machine awake for as long as the session lives: the sleep timer does not know a stress test is going.
    _awake: os::KeepAwake,
    /// Makes closing the console wait until the values are back (see `pbo_os::Busy`).
    _busy: os::Busy,
    work_root: PathBuf,
    whea_since: String,
    names: eventlog::Names,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Passed {
        cycles: u32,
    },
    Failed(String),
    /// A machine check named this core while it was under test.
    FailedByWhea(String),
    /// The test did not run properly (the engine could not start, or the core was not loaded): it says nothing about
    /// the value, so nothing is remembered about it.
    Inconclusive(String),
    Cancelled,
}

impl<'a> Session<'a> {
    pub fn new(
        machine: &'a Machine,
        store: &'a Store,
        log: DailyLog,
        setup: Setup,
        run_id: String,
        smt: bool,
    ) -> Session<'a> {
        let (knowledge, bios) = store.load_knowledge();
        Session {
            machine,
            store,
            log,
            setup,
            extra_legs: Vec::new(),
            run_id,
            started_utc: os::utc_iso(),
            baseline: machine.read_margins().unwrap_or_default(),
            knowledge,
            bios,
            smt,
            transient: pbo_core::transient::Transient::default(),
            events: None,
            _awake: os::KeepAwake::new(),
            _busy: os::Busy::new(),
            work_root: store.root.join("engine-work"),
            whea_since: os::utc_iso(),
            names: eventlog::Names::measure(),
        }
    }

    fn emit(&self, event: Event) {
        if let Some(tx) = &self.events {
            let _ = tx.send(event);
        }
    }

    pub fn say(&mut self, text: &str) {
        println!("{text}");
        self.emit(Event::Line(text.to_string()));
        let _ = self.log.line(text);
    }

    pub fn save_knowledge(&self) {
        if let Err(e) = self.store.save_knowledge(&self.knowledge, &self.bios) {
            eprintln!("could not save what was learned: {e}");
        }
    }

    fn core_of(&self, event: &WheaEvent) -> Option<usize> {
        self.names.core_of(event)
    }

    /// Runs the engine on `core` for `seconds` at the value the core holds now (`margin`, for the state file and
    /// the log). Returns at the first failure.
    pub fn test_core(&mut self, core: usize, margin: i32, seconds: u64) -> Outcome {
        let mut legs = vec![self.setup.clone()];
        legs.extend(self.extra_legs.clone());
        let mut cycles = 0;
        for leg in &legs {
            match self.test_leg(core, margin, seconds, leg) {
                Outcome::Passed { cycles: c } => cycles += c,
                // The first step that does not pass ends the test of this value: nothing after it is run.
                other => return other,
            }
        }
        Outcome::Passed { cycles }
    }

    /// One step of the test: one engine on the core for `seconds`.
    fn test_leg(&mut self, core: usize, margin: i32, seconds: u64, setup: &Setup) -> Outcome {
        let threads = if self.smt {
            self.machine.threads_of(core)
        } else {
            self.machine.threads_of(core).into_iter().take(1).collect()
        };

        let state = RunState {
            run_id: self.run_id.clone(),
            core: core as u32,
            margin,
            baseline: self.baseline.clone(),
            started_utc: self.started_utc.clone(),
        };
        let _ = self.store.write_run_state(&state);

        let mut stress = match Stress::start(setup, &self.work_root, core, &threads) {
            Ok(s) => s,
            Err(e) => {
                return Outcome::Inconclusive(format!(
                    "could not start {}: {e}",
                    setup.kind.name()
                ));
            }
        };
        let auto = seconds == 0;
        let _ = self.log.line(&format!(
            "core {core}: {} on logical {:?} at {margin} for {}",
            setup.kind.name(),
            threads,
            if auto {
                "one run through the work list (60 min at most)".to_string()
            } else {
                format!("{seconds} s")
            }
        ));

        self.emit(Event::Progress {
            core,
            margin,
            seconds,
        });
        // How the load is interrupted: a pause on the loop's clock, or pulses on a clock of their own.
        use pbo_core::transient::{Transient, BURST_IDLE_MS, BURST_LOAD_MS};
        let periodic = match self.transient {
            Transient::Periodic { every_s, pause_s } => Some((
                Duration::from_secs(every_s.into()),
                Duration::from_secs(pause_s.into()),
            )),
            _ => None,
        };
        if self.transient != Transient::Off {
            let _ = self.log.line(&format!(
                "core {core}: load changes: {}",
                self.transient.to_text()
            ));
        }
        // Auto: until the engine has been through its work list once, and at most the cap.
        let limit = if auto {
            pbo_core::engine::AUTO_RUNTIME_CAP_SECONDS
        } else {
            seconds
        };
        let begin = Instant::now();
        let cpu_begin = stress.cpu_time();
        let mut paused = Duration::ZERO;
        let mut next_pause = periodic.map(|(every, _)| every);
        let pulser = match self.transient {
            Transient::Bursts => Some(crate::pulse::Pulser::start(
                stress.suspender(),
                Duration::from_millis(BURST_LOAD_MS.into()),
                Duration::from_millis(BURST_IDLE_MS.into()),
            )),
            Transient::Game => {
                let seed = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(1, |d| d.as_nanos() as u64);
                let mut rhythm = pbo_core::transient::GamePattern::new(seed);
                Some(crate::pulse::Pulser::start_rhythm(
                    stress.suspender(),
                    move || {
                        let (load, idle) = rhythm.next_slice();
                        (
                            Duration::from_millis(load.into()),
                            Duration::from_millis(idle.into()),
                        )
                    },
                ))
            }
            _ => None,
        };
        let mut next_sample = Instant::now();
        let mut next_whea = Instant::now() + Duration::from_secs(10);
        let outcome = loop {
            if os::cancelled() {
                break Outcome::Cancelled;
            }
            std::thread::sleep(Duration::from_secs(1));

            // The pause: frozen for a moment, so the core drops to idle and has to boost back up. It is not counted
            // as time the engine had to use the core.
            if let (Some((every, pause)), Some(due)) = (periodic, next_pause) {
                if begin.elapsed() >= due {
                    stress.suspend();
                    std::thread::sleep(pause);
                    stress.resume();
                    paused += pause;
                    next_pause = Some(begin.elapsed() + every);
                }
            }

            let failures: Vec<Failure> = stress.poll();
            if let Some(f) = failures.first() {
                break Outcome::Failed(f.text.clone());
            }

            if Instant::now() >= next_sample {
                next_sample += Duration::from_secs(5);
                let s = self.machine.sample();
                self.emit(Event::Sample(s.clone()));
                let f = |v: Option<f32>, unit: &str| {
                    v.map_or("n/a".to_string(), |x| format!("{x:.1}{unit}"))
                };
                let volts = s
                    .core_volts
                    .map_or("n/a".to_string(), |v| format!("{v:.3}V"));
                let _ = self.log.line(&format!(
                    "telemetry core {core}: temp {} ppt {} tdc {} edc {} vcore {volts}",
                    f(s.temperature, "C"),
                    f(s.ppt_watts, "W"),
                    f(s.tdc_amps, "A"),
                    f(s.edc_amps, "A"),
                ));
            }

            if Instant::now() >= next_whea {
                next_whea += Duration::from_secs(10);
                if let Some(o) = self.check_whea(core) {
                    break o;
                }
            }

            let by_cycle = auto && stress.cycles() >= 1;
            if by_cycle || begin.elapsed() >= Duration::from_secs(limit) {
                // One last look: an event written in the final seconds still counts.
                if let Some(o) = self.check_whea(core) {
                    break o;
                }
                if by_cycle {
                    let _ = self.log.line(&format!(
                        "core {core}: the whole work list has run once, after {:.0} s",
                        begin.elapsed().as_secs_f64()
                    ));
                }
                break Outcome::Passed {
                    cycles: stress.cycles(),
                };
            }
        };

        // The share of the time the load was let run, as the pulser measured it (an even pattern or not).
        let share = pulser.as_ref().and_then(|p| p.share());
        // Stop the pulses first, so the engine is released and nothing touches the handle after it is closed.
        drop(pulser);

        // Did the engine load the core? A test that did not run proves nothing (invariant I5). The time it was
        // frozen on purpose does not count against it: it is measured against the share of the time it should run.
        let used = stress.cpu_time().saturating_sub(cpu_begin).as_secs_f64();
        let wall = ((begin.elapsed().saturating_sub(paused)).as_secs_f64()
            * share.unwrap_or_else(|| self.transient.duty_cycle()))
        .max(0.001);
        stress.stop();
        if let Outcome::Passed { .. } = outcome {
            let load = used / wall / threads.len() as f64;
            let _ = self.log.line(&format!(
                "core {core}: engine used {:.0} % of the core",
                load * 100.0
            ));
            if load < 0.5 {
                return Outcome::Inconclusive(format!(
                    "the engine used only {:.0} % of the core: this test proved nothing",
                    load * 100.0
                ));
            }
        }
        outcome
    }

    /// Machine checks since the last look. A check that names the core under test is a failure; checks on other
    /// cores are logged and left to whoever is testing them.
    fn check_whea(&mut self, core: usize) -> Option<Outcome> {
        let events = eventlog::since(&self.whea_since);
        if events.is_empty() {
            return None;
        }
        if let Some(last) = events.last() {
            self.whea_since = last.time.clone();
        }
        let mut result = None;
        for e in &events {
            let named = self.core_of(e);
            let _ = self.log.important(&format!(
                "WHEA {} ({}) at {}: processor APIC id {}, core {}",
                e.id,
                if e.warning { "warning" } else { "error" },
                e.time,
                e.apic_id.map_or("?".to_string(), |a| a.to_string()),
                named.map_or("not known".to_string(), |c| c.to_string()),
            ));
            let why = match whea::blame(e, named, core) {
                whea::Blame::TestedCore => Some(format!("machine check {} on core {core}", e.id)),
                whea::Blame::Unattributed => Some(format!(
                    "machine check {} that names no processor, counted against core {core} (the only one loaded)",
                    e.id
                )),
                whea::Blame::OtherCore | whea::Blame::Ignored => None,
            };
            if let (Some(why), true) = (why, result.is_none()) {
                result = Some(Outcome::FailedByWhea(why));
            }
        }
        result
    }

    pub fn header(machine: &Machine, bios: &str, margins: &[i32]) -> Vec<String> {
        let values: Vec<String> = margins.iter().map(|m| m.to_string()).collect();
        vec![
            format!("pbo {}", env!("CARGO_PKG_VERSION")),
            format!("processor: {} ({})", machine.brand, machine.family.name),
            format!("BIOS: {}", if bios.is_empty() { "unknown" } else { bios }),
            format!("values at start: {}", values.join(" ")),
        ]
    }

    /// Which core a machine check names.
    pub fn names(&self) -> &eventlog::Names {
        &self.names
    }

    pub fn engine_kind(&self) -> EngineKind {
        self.setup.kind
    }
}

/// A setup for a session that tests nothing (`pbo resume`): no program is started with it.
pub fn no_engine_setup() -> Setup {
    Setup {
        kind: EngineKind::Prime95,
        exe: PathBuf::new(),
        fft: pbo_core::engine::FftPreset::Smallest,
        mode: pbo_core::engine::Prime95Mode::Avx2,
        seconds_per_test: 60,
        yc_set: pbo_core::engine::YcSet::default(),
        yc_tests: None,
        yc_memory_mb: None,
    }
}

pub fn default_setup(machine: &Machine, kind: EngineKind) -> Result<Setup, String> {
    let generation = match machine.family.generation {
        pbo_core::families::Generation::ZenOne => 1,
        pbo_core::families::Generation::Zen2 => 2,
        pbo_core::families::Generation::Zen3 => 3,
        pbo_core::families::Generation::Zen4 => 4,
        pbo_core::families::Generation::Zen5 => 5,
        pbo_core::families::Generation::Unknown => 0,
    };
    let exe = match kind {
        EngineKind::Prime95 => stress::find_prime95(),
        EngineKind::YCruncher => stress::find_ycruncher(generation),
    }
    .ok_or_else(|| {
        format!(
            "{} was not found. Put it in an `engines` folder next to the program (engines\\prime95\\prime95.exe, engines\\ycruncher\\...), or set PBO_ENGINES.",
            kind.name()
        )
    })?;
    Ok(Setup {
        kind,
        exe,
        fft: pbo_core::engine::FftPreset::Smallest,
        mode: pbo_core::engine::Prime95Mode::Avx2,
        seconds_per_test: 60,
        yc_set: pbo_core::engine::YcSet::default(),
        yc_tests: None,
        yc_memory_mb: None,
    })
}
