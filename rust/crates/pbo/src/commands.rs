//! `pbo run`, `pbo tune`, `pbo resume`: the commands that test cores.
//!
//! `run` writes nothing to the processor: it tests the cores at the values they hold. `tune` searches each core's
//! value and is the only command that moves values, always through [`Machine::set_margin`] and its read-back, and it
//! puts the values back if it is cancelled. `resume` is what runs at logon after a crash.

use std::collections::BTreeMap;

use pbo_core::campaign;
use pbo_core::interrupted;
use pbo_core::knowledge::{self, Knowledge};
use pbo_core::search::{self, Rig, TestResult};
use pbo_core::tuner::{self, Mode};
use pbo_os as os;

use crate::dailylog::{self, DailyLog};
use crate::machine::Machine;
use crate::session::{self, Event, Outcome, Session};
use crate::store::{self, Store};
use crate::stress::EngineKind;

#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    pub seconds: u64,
    pub cores: Option<Vec<usize>>,
    pub engine: EngineKind,
    pub smt: bool,
    pub guardband: i32,
    /// Where the search of a core that holds nothing begins, in counts below zero; none: the family's suggestion.
    pub start: Option<u32>,
    pub mode: Mode,
    pub fft: Option<pbo_core::engine::FftPreset>,
    pub prime_mode: pbo_core::engine::Prime95Mode,
    /// Further steps per core after the first.
    pub then: Vec<pbo_app::plan::Step>,
    /// How many times a test of the held values goes round the cores.
    pub rounds: u32,
    /// The order the cores are tested in.
    pub order: pbo_core::order::CoreOrder,
    /// What a test of the held values does when a core fails.
    pub on_error: pbo_core::onerror::OnError,
    /// A sound at an error and at the end of the run.
    pub notify: bool,
    /// Which algorithms y-cruncher runs.
    pub yc_set: pbo_core::engine::YcSet,
    /// How the load is interrupted so the core has to boost back up.
    pub transient: pbo_core::transient::Transient,
    /// Seconds of rest between two cores, for them to cool down.
    pub pause: u64,
    /// After a crash, carry on with Auto by itself (`pbo resume --continue`).
    pub continue_auto: bool,
    /// The countdown before that, in seconds.
    pub wait: u64,
    /// y-cruncher's algorithms of one's own, by their tags; none: the set that was chosen.
    pub yc_tests: Option<Vec<String>>,
    /// The memory y-cruncher's stress test gets for each logical processor, in MiB; none: 64.
    pub yc_memory_mb: Option<u32>,
}

pub(crate) fn parse(args: &[String]) -> Result<Options, String> {
    let mut o = Options {
        seconds: 60,
        cores: None,
        engine: EngineKind::Prime95,
        smt: false,
        guardband: tuner::DEFAULT_GUARDBAND,
        start: None,
        mode: Mode::Coarse,
        fft: None,
        prime_mode: pbo_core::engine::Prime95Mode::Avx2,
        then: Vec::new(),
        rounds: 1,
        order: pbo_core::order::CoreOrder::default(),
        transient: pbo_core::transient::Transient::default(),
        on_error: pbo_core::onerror::OnError::default(),
        notify: true,
        yc_set: pbo_core::engine::YcSet::default(),
        pause: 0,
        continue_auto: false,
        wait: crate::autorun::DEFAULT_COUNTDOWN_SECONDS,
        yc_tests: None,
        yc_memory_mb: None,
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = |name: &str| it.next().ok_or(format!("{name} needs a value"));
        match a.as_str() {
            "--seconds" => {
                let text = value("--seconds")?;
                o.seconds = if text.eq_ignore_ascii_case("auto") {
                    0
                } else {
                    text.parse()
                        .map_err(|_| "--seconds takes a number of seconds, or auto")?
                };
            }
            "--on-error" => {
                let text = value("--on-error")?;
                o.on_error = pbo_core::onerror::OnError::from_id(text).ok_or(format!(
                    "--on-error takes continue, skip or stop, not '{text}'"
                ))?;
            }
            "--quiet" => o.notify = false,
            "--yc-set" => {
                let text = value("--yc-set")?;
                o.yc_set = pbo_core::engine::YcSet::from_id(text).ok_or(format!(
                    "--yc-set takes curve-optimizer or fast-discovery, not '{text}'"
                ))?;
            }
            "--cores" => {
                let list = value("--cores")?;
                o.cores = Some(
                    list.split(',')
                        .map(|c| {
                            c.trim()
                                .parse::<usize>()
                                .map_err(|_| format!("'{c}' is not a core number"))
                        })
                        .collect::<Result<_, _>>()?,
                );
            }
            "--engine" => {
                let n = value("--engine")?;
                o.engine = EngineKind::from_name(n)
                    .ok_or(format!("unknown engine '{n}' (prime95 or ycruncher)"))?;
            }
            "--smt" => o.smt = true,
            "--start" => {
                let n: u32 = value("--start")?
                    .parse()
                    .ok()
                    .filter(|n| (1..=30).contains(n))
                    .ok_or("--start takes a number from 1 to 30")?;
                o.start = Some(n);
            }
            "--pause" => {
                o.pause = value("--pause")?
                    .parse()
                    .ok()
                    .filter(|n| *n <= 3600)
                    .ok_or("--pause takes a number of seconds, up to 3600")?;
            }
            "--continue" => o.continue_auto = true,
            "--wait" => {
                o.wait = value("--wait")?
                    .parse()
                    .ok()
                    .filter(|n| *n <= 3600)
                    .ok_or("--wait takes a number of seconds, up to 3600")?;
            }
            "--yc-tests" => {
                let list = value("--yc-tests")?;
                let mut tags = Vec::new();
                for part in list.split(',').map(str::trim).filter(|p| !p.is_empty()) {
                    tags.push(
                        pbo_core::engine::ycruncher_tag(part)
                            .ok_or(format!(
                                "'{part}' is not a y-cruncher stress test (one of: {})",
                                pbo_core::engine::YCRUNCHER_TAGS.join(", ")
                            ))?
                            .to_string(),
                    );
                }
                if tags.is_empty() {
                    return Err("--yc-tests needs at least one test".into());
                }
                o.yc_tests = Some(tags);
            }
            "--yc-memory" => {
                o.yc_memory_mb = Some(
                    value("--yc-memory")?
                        .parse()
                        .ok()
                        .filter(|n| (16..=4096).contains(n))
                        .ok_or("--yc-memory takes MiB for each logical processor, 16 to 4096")?,
                );
            }
            "--guardband" => {
                o.guardband = value("--guardband")?
                    .parse()
                    .map_err(|_| "--guardband takes a number")?
            }
            "--fine" => o.mode = Mode::Fine,
            "--step" => {
                let n: u32 = value("--step")?
                    .parse()
                    .ok()
                    .filter(|n| (1..=tuner::MAX_STEP as u32).contains(n))
                    .ok_or(format!(
                        "--step takes a number from 1 to {}",
                        tuner::MAX_STEP
                    ))?;
                o.mode = pbo_app::plan::Search::stepping(n)
                    .mode()
                    .unwrap_or(Mode::Coarse);
            }
            "--order" => {
                let name = value("--order")?;
                o.order = pbo_core::order::CoreOrder::from_id(name).ok_or(format!(
                    "--order takes sequential, alternate or random, not '{name}'"
                ))?;
            }
            "--transient" => {
                let text = value("--transient")?;
                o.transient = pbo_core::transient::Transient::from_text(text).ok_or(format!(
                    "--transient takes off, bursts, periodic or periodic:EVERY:PAUSE, not '{text}'"
                ))?;
            }
            "--rounds" => {
                o.rounds = value("--rounds")?
                    .parse::<u32>()
                    .ok()
                    .filter(|n| (1..=50).contains(n))
                    .ok_or("--rounds takes a number from 1 to 50")?;
            }
            "--then" => {
                let list = value("--then")?;
                for part in list.split('+').filter(|p| !p.is_empty()) {
                    o.then.push(
                        pbo_app::plan::Step::from_text(part)
                            .ok_or(format!("'{part}' is not a step (engine:mode:fft)"))?,
                    );
                }
            }
            "--mode" => {
                o.prime_mode = match value("--mode")?.to_ascii_lowercase().as_str() {
                    "sse" => pbo_core::engine::Prime95Mode::Sse,
                    "avx" => pbo_core::engine::Prime95Mode::Avx,
                    "avx2" => pbo_core::engine::Prime95Mode::Avx2,
                    "avx512" => pbo_core::engine::Prime95Mode::Avx512,
                    other => {
                        return Err(format!("unknown mode '{other}' (sse, avx, avx2 or avx512)"))
                    }
                }
            }
            "--fft" => {
                let n = value("--fft")?;
                o.fft = Some(
                    pbo_core::engine::FftPreset::from_name(n)
                        .ok_or(format!("unknown FFT preset '{n}'"))?,
                );
            }
            other => return Err(format!("unknown option {other}")),
        }
    }
    Ok(o)
}

pub(crate) fn open_machine() -> Result<Machine, i32> {
    Machine::open().map_err(|e| {
        eprintln!("{e}");
        1
    })
}

pub(crate) fn chosen_cores(
    machine: &Machine,
    requested: &Option<Vec<usize>>,
) -> Result<Vec<usize>, String> {
    match requested {
        None => Ok((0..machine.cores).collect()),
        Some(list) => {
            if let Some(bad) = list.iter().find(|c| **c >= machine.cores) {
                Err(format!(
                    "core {bad} does not exist: this processor has {} cores (0 to {})",
                    machine.cores,
                    machine.cores - 1
                ))
            } else {
                Ok(list.clone())
            }
        }
    }
}

fn start_session<'a>(
    machine: &'a Machine,
    store: &'a Store,
    o: &Options,
    events: Option<std::sync::mpsc::Sender<Event>>,
) -> Result<Session<'a>, String> {
    start_session_with(machine, store, o, events, true)
}

/// A session; `needs_engine` false for what tests nothing (`pbo resume`): putting the proven values back after a crash must
/// not depend on a test program being where it was.
fn start_session_with<'a>(
    machine: &'a Machine,
    store: &'a Store,
    o: &Options,
    events: Option<std::sync::mpsc::Sender<Event>>,
    needs_engine: bool,
) -> Result<Session<'a>, String> {
    let mut setup = match session::default_setup(machine, o.engine) {
        Ok(s) => s,
        Err(_) if !needs_engine => session::no_engine_setup(),
        Err(e) => return Err(e),
    };
    if let Some(fft) = o.fft {
        setup.fft = fft;
    }
    setup.mode = o.prime_mode;
    setup.yc_set = o.yc_set;
    setup.yc_tests = o.yc_tests.clone();
    setup.yc_memory_mb = o.yc_memory_mb;
    let mut extra = Vec::new();
    for step in o.then.iter().filter(|_| needs_engine) {
        let kind = match step.engine {
            pbo_app::plan::Engine::Prime95 => EngineKind::Prime95,
            pbo_app::plan::Engine::YCruncher => EngineKind::YCruncher,
        };
        let mut leg = session::default_setup(machine, kind)?;
        leg.mode = step.mode;
        leg.fft = step.fft;
        leg.yc_set = o.yc_set;
        leg.yc_tests = o.yc_tests.clone();
        leg.yc_memory_mb = o.yc_memory_mb;
        extra.push(leg);
    }
    let run_id = dailylog::new_run_id();
    let bios = os::bios_version().unwrap_or_default();
    let set_aside = store.set_aside_if_bios_changed(&bios);
    let margins = machine.read_margins().unwrap_or_default();
    let header = Session::header(machine, &bios, &margins);
    let log = DailyLog::open(
        store.logs_dir(),
        &run_id,
        dailylog::DEFAULT_BOUNDARY_HOUR,
        &header,
    )
    .map_err(|e| format!("log: {e}"))?;
    println!("Log: {}", log.path.display());
    let mut s = Session::new(machine, store, log, setup, run_id, o.smt);
    s.extra_legs = extra;
    s.transient = o.transient;
    s.bios = bios;
    s.events = events;
    if let Some((old, new)) = set_aside {
        s.say(&format!(
            "The BIOS changed from {old} to {new} since the values were learned: what was learned is set aside (kept in runs\\core_knowledge.bios-*.json) and the cores are measured again."
        ));
    }
    Ok(s)
}

// ── pbo run ───────────────────────────────────────────────────────────

/// Tests the cores at the values they hold. Changes nothing on the processor.
pub fn run(args: &[String]) -> i32 {
    let o = match parse(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    os::handle_ctrl_c();
    run_with(&o, None)
}

/// [`run`] for a caller that already has its options, with an optional channel the progress is sent to.
pub fn run_with(o: &Options, events: Option<std::sync::mpsc::Sender<Event>>) -> i32 {
    let machine = match open_machine() {
        Ok(m) => m,
        Err(c) => return c,
    };
    let store = Store::beside_program();
    let cores = match chosen_cores(&machine, &o.cores) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    let mut session = match start_session(&machine, &store, o, events) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    report_interrupted(&mut session);

    session.say(&format!(
        "Testing cores {:?} at the values they hold, {} each, with {}. Nothing is changed on the processor.",
        cores,
        if o.seconds == 0 {
            "one run through the work list".to_string()
        } else {
            format!("{} s", o.seconds)
        },
        session.engine_kind().name()
    ));
    let mut failed = 0;
    let mut cancelled = false;
    let mut stopped = false;
    let mut policy = pbo_core::onerror::ErrorPolicy::new(o.on_error);
    let mut tested_one = false;
    for round in 1..=o.rounds.max(1) {
        if o.rounds > 1 {
            session.say(&format!("round {round} of {}", o.rounds));
        }
        let in_order = policy.remaining(&pbo_core::order::arrange(o.order, &cores, order_seed()));
        if o.order != pbo_core::order::CoreOrder::Sequential {
            session.say(&format!("order of the cores: {in_order:?}"));
        }
        for core in &in_order {
            if tested_one {
                rest_between(&mut session, o.pause);
            }
            tested_one = true;
            let core = *core;
            let held = match machine.read_margin(core) {
                Ok(m) => m,
                Err(e) => {
                    session.say(&format!("core {core}: cannot read its value ({e})"));
                    failed += 1;
                    continue;
                }
            };
            let outcome = session.test_core(core, held, o.seconds);
            if !record(&mut session, core, held, &outcome) {
                failed += 1;
            }
            // Kept at once: a crash later in a long run must not take what was learned before it.
            session.save_knowledge();
            if matches!(outcome, Outcome::Failed(_) | Outcome::FailedByWhea(_)) {
                policy.failed(core);
                if o.notify {
                    os::sound(os::Sound::Error);
                }
                if policy.stop_requested() {
                    session.say("stopping at the first error, as asked");
                    stopped = true;
                    break;
                }
            }
            if outcome == Outcome::Cancelled {
                cancelled = true;
                break;
            }
        }
        if cancelled || stopped {
            break;
        }
    }
    session.store.clear_run_state();
    session.save_knowledge();
    if o.notify && !cancelled && !stopped {
        os::sound(os::Sound::Done);
    }
    let _ = session.log.important(if cancelled {
        "run ended: cancelled"
    } else if failed == 0 {
        "run ended: all cores passed"
    } else {
        "run ended: failures"
    });
    if failed == 0 {
        0
    } else {
        1
    }
}

/// Rest between two cores, so that the next one starts cool (`--pause N`). It ends at once when the run is cancelled.
fn rest_between(session: &mut Session, seconds: u64) {
    if seconds == 0 {
        return;
    }
    session.say(&format!("resting {seconds} s before the next core"));
    for _ in 0..seconds {
        if os::cancelled() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

/// Something different every time, for a shuffle of the cores.
fn order_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1)
}

/// Says what happened to a core and puts it into the memory. Returns whether the core passed.
fn record(session: &mut Session, core: usize, margin: i32, outcome: &Outcome) -> bool {
    match outcome {
        Outcome::Passed { cycles } => {
            session.say(&format!(
                "core {core:2}: held at {margin} ({cycles} full sweeps)"
            ));
            knowledge::record_pass(&mut session.knowledge, core as u32, margin);
            true
        }
        Outcome::Failed(why) | Outcome::FailedByWhea(why) => {
            session.say(&format!("core {core:2}: FAILED at {margin}: {why}"));
            knowledge::record_failure(&mut session.knowledge, core as u32, margin);
            false
        }
        Outcome::Inconclusive(why) => {
            session.say(&format!(
                "core {core:2}: not tested at {margin}: {why}. Nothing is remembered about this value."
            ));
            false
        }
        Outcome::Cancelled => {
            session.say(&format!("core {core:2}: cancelled"));
            true
        }
    }
}

// ── Recovery after an interrupted run ────────────────────────────────

/// If the last run did not end: a crash when the computer started again since, a closed window otherwise. A crash is
/// written into the memory against the value that was running; a closed window is not (invariant I9).
fn report_interrupted(session: &mut Session) {
    let Some(state) = session.store.read_run_state() else {
        return;
    };
    let written = session.store.run_state_written();
    let started = store::now_unix() - (os::uptime_ms() / 1000) as i64;
    match interrupted::system_restarted(written, started) {
        Some(true) => {
            session.say(&format!(
                "The last run (started {}) did not end and the computer has restarted since: core {} crashed at {}.",
                state.started_utc, state.core, state.margin
            ));
            knowledge::record_failure(&mut session.knowledge, state.core, state.margin);
            session.save_knowledge();
            // Windows writes the fatal checks of one crash together, one for each processor that reported: all of
            // them are said, with the core each names, and whether the core that was under test is among them.
            let crash = pbo_core::whea::crashes(&crate::eventlog::fatal(50), 120)
                .pop()
                .unwrap_or_default();
            if let Some(first) = crash.first() {
                let names = session.names();
                let named: Vec<String> = crash.iter().map(|e| names.describe(e)).collect();
                let among = crash
                    .iter()
                    .any(|e| names.core_of(e) == Some(state.core as usize));
                let known = crash.iter().any(|e| names.core_of(e).is_some());
                session.say(&format!(
                    "Windows logged {} fatal machine check(s) at {}: {}.{}",
                    crash.len(),
                    first.time,
                    named.join(", "),
                    match (known, among) {
                        (_, true) => format!(" Core {} is among them.", state.core),
                        (true, false) => format!(" Core {} is not among them.", state.core),
                        (false, _) => String::new(),
                    }
                ));
            }
        }
        Some(false) => {
            session.say(&format!(
                "The last run (started {}) did not end, but the computer did not restart: the program was closed. Nothing is held against core {}.",
                state.started_utc, state.core
            ));
        }
        None => {}
    }
    session.store.clear_run_state();
}

/// The extra headroom for the cores the processor ranks highest.
fn extra_headroom(preferred: &std::collections::BTreeSet<usize>, core: usize) -> i32 {
    if preferred.contains(&core) {
        tuner::PREFERRED_CORE_EXTRA_GUARDBAND
    } else {
        0
    }
}

/// After a crash: every core gets its proven value back before anything else happens. The core that crashed is
/// taken back above its failure, with headroom.
fn restore_proven(session: &mut Session, guardband: i32) {
    let (_, shallowest) = session.machine.allowed_range();
    let chip_limit = session.machine.allowed_range().0;
    let preferred = session.machine.preferred_cores();
    for core in 0..session.machine.cores {
        let guardband = guardband + extra_headroom(&preferred, core);
        let k = session.knowledge.get(&(core as u32)).copied();
        let target = match k {
            Some(k) if k.worst_failed.is_some() => {
                Some(campaign::back_off_to(&k, chip_limit, guardband))
            }
            Some(k) => k.best_passed.map(|p| tuner::apply_guardband(p, guardband)),
            None => None,
        };
        if let Some(t) = target {
            let t = t.min(shallowest);
            match session.machine.set_margin(core, t) {
                Ok(held) if held == t => session.say(&format!("core {core:2}: put back to {t}")),
                Ok(held) => session.say(&format!("core {core:2}: asked for {t}, holds {held}")),
                Err(e) => session.say(&format!("core {core:2}: could not put back {t}: {e}")),
            }
        }
    }
}

// ── pbo tune ──────────────────────────────────────────────────────────

/// Before anything is written on a processor family nobody has checked, the write path proves itself: each core's own
/// value is written back and read.
fn write_path_ok(machine: &Machine) -> Result<(), String> {
    if !pbo_core::families::needs_selftest(&machine.family) {
        return Ok(());
    }
    println!(
        "{} has not been checked on hardware; testing the write path first (it changes nothing)...",
        machine.family.name
    );
    for core in 0..machine.cores {
        match machine.selftest(core) {
            Ok((a, b)) if a == b => {}
            Ok((a, b)) => return Err(format!("core {core}: wrote {a}, read back {b}: the write path does not work on this processor")),
            Err(e) => return Err(e),
        }
    }
    println!("The write path works.");
    Ok(())
}

/// Puts a core's value back unless the tuning of that core finished.
struct Safe<'m> {
    machine: &'m Machine,
    core: usize,
    value: i32,
    armed: bool,
}

impl Drop for Safe<'_> {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.machine.set_margin(self.core, self.value);
        }
    }
}

/// The search's view of the real world: the processor, the engine and the memory.
struct TuneRig<'s, 'a> {
    session: &'s mut Session<'a>,
    machine: &'a Machine,
    seconds: u64,
    /// What the last test said, for `record`.
    last: Option<Outcome>,
}

impl Rig for TuneRig<'_, '_> {
    fn set(&mut self, core: usize, margin: i32) -> Result<i32, String> {
        self.machine.set_margin(core, margin)
    }

    fn test(&mut self, core: usize, margin: i32) -> TestResult {
        let outcome = self.session.test_core(core, margin, self.seconds);
        let result = match &outcome {
            Outcome::Passed { .. } => TestResult::Pass,
            Outcome::Failed(_) | Outcome::FailedByWhea(_) => TestResult::Fail,
            Outcome::Inconclusive(_) => TestResult::Inconclusive,
            Outcome::Cancelled => TestResult::Cancelled,
        };
        self.last = Some(outcome);
        result
    }

    fn record(&mut self, core: usize, margin: i32, _result: TestResult) {
        if let Some(outcome) = self.last.take() {
            record(self.session, core, margin, &outcome);
        }
        self.session.save_knowledge();
    }

    fn knowledge(&self, core: usize) -> pbo_core::knowledge::CoreKnowledge {
        self.session
            .knowledge
            .get(&(core as u32))
            .copied()
            .unwrap_or_default()
    }

    fn say(&mut self, text: &str) {
        self.session.say(text);
    }

    fn cancelled(&self) -> bool {
        os::cancelled()
    }
}

pub fn tune(args: &[String]) -> i32 {
    let o = match parse(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    os::handle_ctrl_c();
    tune_with(&o, None)
}

pub fn tune_with(o: &Options, events: Option<std::sync::mpsc::Sender<Event>>) -> i32 {
    let machine = match open_machine() {
        Ok(m) => m,
        Err(c) => return c,
    };
    if !machine.family.curve_optimizer {
        eprintln!(
            "{} has no Curve Optimizer: there is nothing to tune.",
            machine.family.name
        );
        return 1;
    }
    if let Err(e) = write_path_ok(&machine) {
        eprintln!("{e}");
        return 1;
    }
    let store = Store::beside_program();
    let cores = match chosen_cores(&machine, &o.cores) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    let mut session = match start_session(&machine, &store, o, events) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    report_interrupted(&mut session);

    let (deepest, shallowest) = machine.allowed_range();
    let model_limit = machine.family.start_depth;
    let held_now = machine.read_margins().unwrap_or_default();
    let preferred = machine.preferred_cores();
    if !preferred.is_empty() {
        let list: Vec<String> = preferred.iter().map(|c| c.to_string()).collect();
        session.say(&format!(
            "Preferred cores per CPPC: {} (they keep {} extra step(s) of headroom).",
            list.join(", "),
            tuner::PREFERRED_CORE_EXTRA_GUARDBAND
        ));
    }
    session.say(&format!(
        "Tuning cores {:?}: {} s per test, steps of {}, headroom {} step(s), range {deepest} to {shallowest}.",
        cores,
        o.seconds,
        tuner::step_size(o.mode),
        o.guardband
    ));

    let mut failed = 0;
    let mut results: BTreeMap<usize, i32> = BTreeMap::new();
    let mut tested_one = false;
    let cores = pbo_core::order::arrange(o.order, &cores, order_seed());
    if o.order != pbo_core::order::CoreOrder::Sequential {
        session.say(&format!("order of the cores: {cores:?}"));
    }
    for core in cores {
        if tested_one {
            rest_between(&mut session, o.pause);
        }
        tested_one = true;
        let proven = session
            .knowledge
            .get(&(core as u32))
            .copied()
            .unwrap_or_default();
        let gb = o.guardband + extra_headroom(&preferred, core);
        let mut safe = Safe {
            machine: &machine,
            core,
            value: proven
                .best_passed
                .map_or(held_now.get(core).copied().unwrap_or(0), |p| {
                    tuner::apply_guardband(p, gb)
                }),
            armed: true,
        };
        let params = search::Params {
            model_limit,
            held_before: held_now.clone(),
            deepest,
            shallowest,
            mode: o.mode,
            guardband: gb,
            suggested_start: o.start.map(|n| -(n as i32)).unwrap_or_else(|| {
                pbo_core::families::suggested_start(&machine.family, machine.x3d)
            }),
            held_now: machine.read_margin(core).unwrap_or(0),
        };
        let found = {
            let mut rig = TuneRig {
                session: &mut session,
                machine: &machine,
                seconds: o.seconds,
                last: None,
            };
            search::search_core(&mut rig, core, &params)
        };
        let locked_at = match found {
            search::Search::Locked(v) => Some(v),
            search::Search::Unfinished(_) => None,
        };

        match locked_at {
            Some(value) => match machine.set_margin(core, value) {
                Ok(held) if held == value => {
                    session.say(&format!("core {core:2}: locked at {value}"));
                    results.insert(core, value);
                    safe.armed = false;
                }
                Ok(held) => {
                    session.say(&format!(
                        "core {core:2}: wanted {value} but the processor holds {held}"
                    ));
                    failed += 1;
                }
                Err(e) => {
                    session.say(&format!("core {core:2}: could not set {value}: {e}"));
                    failed += 1;
                }
            },
            None => {
                failed += 1;
                session.say(&format!(
                    "core {core:2}: not finished; put back to {}",
                    safe.value
                ));
                if os::cancelled() {
                    drop(safe);
                    break;
                }
            }
        }
    }
    session.store.clear_run_state();
    session.save_knowledge();
    let list: Vec<String> = results.iter().map(|(c, v)| format!("{c}:{v}")).collect();
    session.say(&format!(
        "Result: {}",
        if list.is_empty() {
            "nothing locked".into()
        } else {
            list.join(" ")
        }
    ));
    let _ = session.log.important("tune ended");
    if o.notify && !os::cancelled() {
        os::sound(os::Sound::Done);
    }
    if failed == 0 {
        0
    } else {
        1
    }
}

// ── pbo resume ────────────────────────────────────────────────────────

/// What runs at logon: after a crash, every core is put back to what it was proven to hold, and the person is told
/// what happened. It does not start a test by itself; that is a decision the person makes.
pub fn resume(args: &[String]) -> i32 {
    // A continuation that is counting down is stopped with a file; nothing else is done.
    if args.iter().any(|a| a == "--cancel") {
        return crate::autorun::cancel_continuation();
    }
    let o = match parse(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    let machine = match open_machine() {
        Ok(m) => m,
        Err(c) => return c,
    };
    if o.continue_auto {
        // At logon this is a console nobody opened: it goes at once, and the window the person sees is PboStudio.
        pbo_os::hide_own_console();
    }
    let store = Store::beside_program();
    let mut session = match start_session_with(&machine, &store, &o, None, false) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    let had_state = session.store.read_run_state().is_some();
    report_interrupted(&mut session);
    if let Err(e) = write_path_ok(&machine) {
        session.say(&format!("not restoring values: {e}"));
        return 1;
    }
    let known = knowledge::cores_with_known_bad(&session.knowledge);
    session.say(&format!(
        "{known} core(s) have a value on record that failed."
    ));
    if had_state {
        // A run was cut off: what it left is not to be trusted, the proven values are.
        restore_proven(&mut session, o.guardband);
        if o.continue_auto {
            // The proven values are back; Auto carries on by itself after a countdown that can be cancelled.
            drop(session);
            drop(machine);
            crate::autorun::continue_after_crash(o.wait);
        }
        0
    } else if session.store.load_saved_profile().is_some() {
        drop(session);
        drop(machine);
        apply()
    } else {
        0
    }
}

/// `pbo knowledge`: what the program remembers about each core.
pub fn show_knowledge() -> i32 {
    let store = Store::beside_program();
    let (k, bios) = store.load_knowledge();
    if k.is_empty() {
        println!("Nothing is remembered yet.");
        return 0;
    }
    println!(
        "BIOS version it was gathered under: {}",
        if bios.is_empty() {
            "not recorded"
        } else {
            &bios
        }
    );
    print_knowledge(&k);
    0
}

fn print_knowledge(k: &Knowledge) {
    for (core, v) in k {
        println!(
            "core {core:2}: best that held {:>4}   worst that failed {:>4}",
            v.best_passed.map_or("-".into(), |n| n.to_string()),
            v.worst_failed.map_or("-".into(), |n| n.to_string()),
        );
    }
}

// ── pbo set, pbo apply ────────────────────────────────────────────────

/// `pbo set CORE VALUE`: one value, through the checked write path, and what the processor holds afterwards.
pub fn set(args: &[String]) -> i32 {
    let (Some(core), Some(value)) = (
        args.first().and_then(|a| a.parse::<usize>().ok()),
        args.get(1).and_then(|a| a.parse::<i32>().ok()),
    ) else {
        eprintln!("usage: pbo set CORE VALUE   (for example: pbo set 4 -28)");
        return 2;
    };
    let machine = match open_machine() {
        Ok(m) => m,
        Err(c) => return c,
    };
    if core >= machine.cores {
        eprintln!(
            "core {core} does not exist: this processor has {} cores",
            machine.cores
        );
        return 2;
    }
    if let Err(e) = write_path_ok(&machine) {
        eprintln!("{e}");
        return 1;
    }
    match machine.set_margin(core, value) {
        Ok(held) if held == value => {
            println!("core {core}: now {held}");
            0
        }
        Ok(held) => {
            println!("core {core}: asked for {value}, the processor keeps {held}");
            1
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

/// `pbo apply`: writes the saved profile (`runs/co_saved.json`) to the processor, core by core with read-back. This
/// is what runs at logon when a profile is to be applied; it refuses a profile saved for another processor.
pub fn apply() -> i32 {
    let store = Store::beside_program();
    let Some((cpu, margins)) = store.load_saved_profile() else {
        eprintln!("There is no saved profile (runs/co_saved.json).");
        return 1;
    };
    let machine = match open_machine() {
        Ok(m) => m,
        Err(c) => return c,
    };
    // What is safe to write is decided by the same check the vectors pin: another processor gets nothing, a core that
    // does not exist or a value out of range is left out with its reason.
    let (deepest, shallowest) = machine.allowed_range();
    let checked = pbo_core::profile::check(
        &cpu,
        &margins,
        &machine.brand,
        machine.cores as i32,
        deepest,
        shallowest,
    );
    for problem in &checked.problems {
        eprintln!("{problem}");
    }
    if checked.wrong_processor {
        eprintln!("Not applied.");
        return 1;
    }
    if checked.margins.is_empty() {
        eprintln!("Nothing in the saved profile can be written here.");
        return 1;
    }
    if let Err(e) = write_path_ok(&machine) {
        eprintln!("{e}");
        return 1;
    }
    let mut bad = checked.problems.len();
    for (core, value) in &checked.margins {
        let core = *core as usize;
        match machine.set_margin(core, *value) {
            Ok(held) if held == *value => println!("core {core:2}: {held}"),
            Ok(held) => {
                bad += 1;
                println!("core {core:2}: asked for {value}, the processor keeps {held}");
            }
            Err(e) => {
                bad += 1;
                println!("core {core:2}: {e}");
            }
        }
    }
    if bad == 0 {
        0
    } else {
        1
    }
}

// ── pbo autostart ─────────────────────────────────────────────────────

pub const TASK_NAME: &str = "PboStudioResume";

/// The task scheduler, by its full path in the system folder and with a limit on the wait.
fn schtasks(args: &[&str]) -> Option<os::system::Output> {
    os::system::run("schtasks.exe", args, std::time::Duration::from_secs(30))
}

/// `pbo autostart on|off|status`: the task that runs `pbo resume` at logon, with the rights the processor registers
/// need. It runs at logon and never at system start, which would be a surprise (ux.md, question 6).
pub fn autostart(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        Some("status") | None => {
            let exists = schtasks(&["/Query", "/TN", TASK_NAME]).is_some_and(|o| o.success);
            println!(
                "{}",
                if exists {
                    "The logon task is installed."
                } else {
                    "The logon task is not installed."
                }
            );
            0
        }
        Some("on") => {
            let exe = std::env::current_exe()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            let continues = args.get(1).is_some_and(|a| a == "--continue");
            let run = if continues {
                format!("\"{exe}\" resume --continue")
            } else {
                format!("\"{exe}\" resume")
            };
            let out = schtasks(&[
                "/Create", "/TN", TASK_NAME, "/TR", &run, "/SC", "ONLOGON", "/RL", "HIGHEST", "/F",
            ]);
            match out {
                Some(o) if o.success => {
                    println!("Installed: at logon the program puts the proven values back after a crash, and otherwise applies the saved profile.");
                    if continues {
                        println!("After a crash it also carries Auto on by itself: the window opens and counts a minute down, in which it can be stopped (or with `pbo resume --cancel`), at most {} times in a row.", crate::autorun::MAX_RESUMES_IN_A_ROW);
                    }
                    0
                }
                _ => {
                    eprintln!("The task could not be created. Run the program as administrator.");
                    1
                }
            }
        }
        Some("off") => match schtasks(&["/Delete", "/TN", TASK_NAME, "/F"]) {
            Some(o) if o.success => {
                println!("Removed.");
                0
            }
            _ => {
                eprintln!("The task could not be removed (it may not exist, or the program is not elevated).");
                1
            }
        },
        Some(other) => {
            eprintln!("usage: pbo autostart on|off|status (not '{other}')");
            2
        }
    }
}

/// `pbo save`: keeps the values the cores hold now as the profile `pbo resume` applies at logon.
pub fn save() -> i32 {
    let machine = match open_machine() {
        Ok(m) => m,
        Err(c) => return c,
    };
    let margins = match machine.read_margins() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    let t = os::local_time();
    let at = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        t.year, t.month, t.day, t.hour, t.minute, t.second
    );
    match Store::beside_program().save_profile(&machine.brand, &margins, &at) {
        Ok(()) => {
            let list: Vec<String> = margins.iter().map(|m| m.to_string()).collect();
            println!("Saved: {}", list.join(" "));
            0
        }
        Err(e) => {
            eprintln!("could not save: {e}");
            1
        }
    }
}

/// `pbo import [FOLDER]`: takes over what a 1.x install has kept (memory, saved profile, settings, logs) without
/// overwriting anything. With no folder it looks in the program's own folder and its parent.
pub fn import(args: &[String]) -> i32 {
    let store = Store::beside_program();
    let candidates: Vec<std::path::PathBuf> = match args.first() {
        Some(p) => vec![std::path::PathBuf::from(p)],
        None => {
            let here = store::program_dir();
            vec![
                here.clone(),
                here.parent()
                    .map(std::path::Path::to_path_buf)
                    .unwrap_or(here),
            ]
        }
    };
    let mut any = false;
    for from in candidates {
        match store.import_from(&from) {
            Ok(done) if !done.is_empty() => {
                any = true;
                println!("From {}:", from.display());
                for d in done {
                    match d {
                        store::Imported::Copied(n) => println!("  taken over: {n}"),
                        store::Imported::Kept(n) => println!("  kept as it is here: {n}"),
                    }
                }
            }
            Ok(_) => {}
            Err(e) => eprintln!("{}: {e}", from.display()),
        }
    }
    if !any {
        println!(
            r"Nothing to take over was found. Give the folder of the 1.x program: pbo import C:\path\to\PboStudio"
        );
    }
    0
}

impl Options {
    /// What a plan means for the commands: the window, the helper and `pbo plan run` all start from this one place, so
    /// what is shown is what runs.
    pub fn from_plan(plan: &pbo_app::plan::Plan) -> Options {
        Options {
            seconds: u64::from(plan.seconds_per_test),
            cores: if plan.cores.is_empty() {
                None
            } else {
                Some(plan.cores.clone())
            },
            engine: match plan.engine {
                pbo_app::plan::Engine::Prime95 => EngineKind::Prime95,
                pbo_app::plan::Engine::YCruncher => EngineKind::YCruncher,
            },
            smt: plan.smt,
            guardband: plan.guardband,
            start: (plan.start > 0).then_some(plan.start),
            mode: plan.search.mode().unwrap_or(Mode::Coarse),
            fft: Some(plan.fft),
            prime_mode: plan.mode,
            then: plan.then.clone(),
            rounds: plan.rounds,
            order: plan.order,
            transient: plan.transient,
            on_error: plan.on_error,
            notify: plan.notify,
            yc_set: plan.yc_set,
            pause: 0,
            continue_auto: false,
            wait: crate::autorun::DEFAULT_COUNTDOWN_SECONDS,
            yc_tests: None,
            yc_memory_mb: None,
        }
    }
}

fn read_plan(path: &str) -> Result<pbo_app::plan::Plan, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    pbo_app::plan::Plan::from_toml(text.trim_start_matches('\u{feff}'))
        .map_err(|e| format!("{path}: {e}"))
}

/// `pbo plan new FILE | show FILE | command FILE | run FILE`: the plan as a file.
pub fn plan(args: &[String]) -> i32 {
    let usage = || {
        eprintln!("usage: pbo plan new FILE      write the recommended plan to a file to edit");
        eprintln!("       pbo plan show FILE     read a plan file and say what it means");
        eprintln!("       pbo plan command FILE  the command line that does the same");
        eprintln!("       pbo plan run FILE      do what the plan says (a search if it has one)");
        2
    };
    let (Some(sub), Some(path)) = (args.first(), args.get(1)) else {
        return usage();
    };
    match sub.as_str() {
        "new" => {
            if std::path::Path::new(path).exists() {
                eprintln!("{path} exists; it is not overwritten.");
                return 1;
            }
            match std::fs::write(path, pbo_app::plan::Plan::recommended().to_toml()) {
                Ok(()) => {
                    println!("Wrote {path}");
                    0
                }
                Err(e) => {
                    eprintln!("{path}: {e}");
                    1
                }
            }
        }
        "show" | "command" | "run" => {
            let plan = match read_plan(path) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    return 2;
                }
            };
            match sub.as_str() {
                "show" => {
                    for (k, v) in plan.to_pairs() {
                        println!("{k:10} {v}");
                    }
                    println!(
                        "{} part(s) differ from the recommendation",
                        plan.adjusted_parts()
                    );
                    0
                }
                "command" => {
                    println!("{}", plan.command_line());
                    0
                }
                _ => {
                    os::handle_ctrl_c();
                    let o = Options::from_plan(&plan);
                    if plan.search == pbo_app::plan::Search::Off {
                        run_with(&o, None)
                    } else {
                        tune_with(&o, None)
                    }
                }
            }
        }
        _ => usage(),
    }
}

/// `pbo estimate`: how long the campaign still has to run from what this machine holds now, phase by phase. The
/// numbers are calculated from what each core holds and what it may reach, never guessed in prose.
pub fn estimate_cmd(args: &[String]) -> i32 {
    // `--intensity fast|normal|thorough` is the window's own choice; the rest is the usual options.
    let mut intensity = pbo_core::auto::Intensity::default();
    let mut rest: Vec<String> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--intensity" {
            match it
                .next()
                .and_then(|v| pbo_core::auto::Intensity::from_id(v))
            {
                Some(i) => intensity = i,
                None => {
                    eprintln!("--intensity takes fast, normal or thorough");
                    return 2;
                }
            }
        } else {
            rest.push(a.clone());
        }
    }
    let o = match parse(&rest) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    let machine = match open_machine() {
        Ok(m) => m,
        Err(c) => return c,
    };
    let store = Store::beside_program();
    let (knowledge, _) = store.load_knowledge();
    let held_now = machine.read_margins().unwrap_or_default();
    let held: BTreeMap<u32, i32> = held_now
        .iter()
        .enumerate()
        .map(|(c, v)| (c as u32, *v))
        .collect();
    let chip_limit = pbo_core::depth::effective(machine.family.start_depth, None, &held_now, None);
    let cores: Vec<u32> = match chosen_cores(&machine, &o.cores) {
        Ok(c) => c.into_iter().map(|c| c as u32).collect(),
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    // From the phase the campaign is on, the one the window's Auto would start: what is already proven is not counted.
    let next = campaign::recommend(
        &cores,
        &held,
        &knowledge,
        chip_limit,
        true,
        true,
        o.guardband,
    );
    // The steps the window starts from, unless the command line named its own.
    let mode = if rest.iter().any(|a| a == "--fine" || a == "--step") {
        o.mode
    } else {
        intensity.preselected_mode()
    };
    let e = pbo_core::auto::estimate_with(
        intensity,
        mode,
        &cores,
        &held,
        &knowledge,
        chip_limit,
        o.guardband,
        next.stage.phase_number().max(1),
        o.start
            .map(|n| -(n as i32))
            .unwrap_or_else(|| pbo_core::families::suggested_start(&machine.family, machine.x3d)),
    );
    for (phase, minutes) in &e.per_phase {
        println!(
            "phase {phase}: {}  ({})",
            pbo_core::duration::hours((minutes * 60.0) as i64),
            pbo_core::campaign::profile_name_for_phase(*phase)
        );
    }
    println!(
        "total:   {}  at most, for {} core(s), {}",
        pbo_core::duration::hours((e.total_minutes * 60.0) as i64),
        cores.len(),
        intensity.id()
    );
    0
}

#[cfg(test)]
mod plan_tests {
    use super::*;
    use pbo_app::plan::{Engine, Plan, Search};

    fn plans() -> Vec<Plan> {
        let base = Plan::recommended();
        vec![
            base.clone(),
            Plan {
                cores: vec![2, 5],
                ..base.clone()
            },
            Plan {
                engine: Engine::YCruncher,
                search: Search::Off,
                seconds_per_test: 30,
                ..base.clone()
            },
            Plan {
                search: Search::Fine,
                guardband: 5,
                smt: true,
                ..base.clone()
            },
            Plan {
                cores: vec![0],
                engine: Engine::YCruncher,
                search: Search::Coarse,
                guardband: 0,
                ..base.clone()
            },
            Plan {
                seconds_per_test: 3600,
                search: Search::Off,
                smt: true,
                cores: vec![7, 1, 3],
                ..base.clone()
            },
            // What to do at an error, no sound, a range of one's own, the other set, an auto time.
            Plan {
                on_error: pbo_core::onerror::OnError::Stop,
                notify: false,
                yc_set: pbo_core::engine::YcSet::FastDiscovery,
                fft: pbo_core::engine::FftPreset::Custom(36, 248),
                seconds_per_test: 0,
                order: pbo_core::order::CoreOrder::Pairs,
                ..base.clone()
            },
            // The order and the interruptions of the load.
            Plan {
                order: pbo_core::order::CoreOrder::Alternate,
                transient: pbo_core::transient::Transient::Bursts,
                ..base.clone()
            },
            Plan {
                order: pbo_core::order::CoreOrder::Random,
                transient: pbo_core::transient::Transient::Periodic {
                    every_s: 5,
                    pause_s: 1,
                },
                ..base.clone()
            },
            Plan {
                transient: pbo_core::transient::Transient::Off,
                ..base.clone()
            },
            // A ready-made list: two steps and three rounds.
            Plan {
                search: Search::Off,
                seconds_per_test: 360,
                rounds: 3,
                then: vec![pbo_app::plan::Step {
                    engine: Engine::YCruncher,
                    mode: pbo_core::engine::Prime95Mode::Avx2,
                    fft: pbo_core::engine::FftPreset::Smallest,
                }],
                ..base
            },
        ]
    }

    /// What the window shows is what the command line runs: the command line that a plan prints, read back by the
    /// command line's own parser, means exactly what the plan means (ux.md, rule 1: what is shown runs).
    #[test]
    fn the_printed_command_line_means_what_the_plan_means() {
        for plan in plans() {
            let line = plan.command_line();
            let tokens: Vec<String> = line.split(' ').map(str::to_string).collect();
            assert_eq!(tokens[0], "pbo");
            // The verb says whether the processor is written to.
            assert_eq!(tokens[1] == "tune", plan.search != Search::Off, "{line}");
            let parsed = parse(&tokens[2..]).unwrap_or_else(|e| panic!("{line}: {e}"));
            let meant = Options::from_plan(&plan);
            // `--fine` and the mode only matter for a search; a plan without one carries the default.
            let same = Options {
                mode: if plan.search == Search::Off {
                    parsed.mode
                } else {
                    meant.mode
                },
                ..meant.clone()
            };
            // No --fft means the default range, which the plan carries explicitly.
            let norm = |mut o: Options| {
                o.fft = o.fft.or(Some(pbo_core::engine::FftPreset::Smallest));
                o
            };
            assert_eq!(norm(parsed), norm(same), "{line}");
        }
    }

    #[test]
    fn a_plan_file_means_the_same_as_the_plan_it_was_written_from() {
        for plan in plans() {
            let back = Plan::from_toml(&plan.to_toml()).unwrap();
            assert_eq!(Options::from_plan(&back), Options::from_plan(&plan));
            assert_eq!(back.command_line(), plan.command_line());
        }
    }
}

#[cfg(test)]
mod option_tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_rest_between_cores_is_a_number_of_seconds_up_to_an_hour() {
        assert_eq!(parse(&args(&["--pause", "20"])).unwrap().pause, 20);
        assert_eq!(parse(&args(&[])).unwrap().pause, 0);
        assert!(parse(&args(&["--pause", "3601"])).is_err());
        assert!(parse(&args(&["--pause", "soon"])).is_err());
        assert!(parse(&args(&["--pause"])).is_err());
    }

    #[test]
    fn y_cruncher_tests_are_checked_against_its_tags_and_spelled_as_it_wants() {
        let o = parse(&args(&["--yc-tests", "sftv4, vt3,N63"])).unwrap();
        assert_eq!(
            o.yc_tests,
            Some(vec![
                "SFTv4".to_string(),
                "VT3".to_string(),
                "N63".to_string()
            ])
        );
        assert!(parse(&args(&["--yc-tests", "SFTv4,nonsense"])).is_err());
        assert!(parse(&args(&["--yc-tests", ","])).is_err());
        assert_eq!(parse(&args(&[])).unwrap().yc_tests, None);
    }

    #[test]
    fn y_cruncher_memory_has_a_sensible_range() {
        assert_eq!(
            parse(&args(&["--yc-memory", "128"])).unwrap().yc_memory_mb,
            Some(128)
        );
        assert!(parse(&args(&["--yc-memory", "8"])).is_err());
        assert!(parse(&args(&["--yc-memory", "100000"])).is_err());
    }

    #[test]
    fn carrying_on_after_a_crash_has_a_countdown_of_a_minute_unless_told_otherwise() {
        let o = parse(&args(&["--continue"])).unwrap();
        assert!(o.continue_auto);
        assert_eq!(o.wait, crate::autorun::DEFAULT_COUNTDOWN_SECONDS);
        assert_eq!(parse(&args(&["--wait", "5"])).unwrap().wait, 5);
        assert!(!parse(&args(&[])).unwrap().continue_auto);
        assert!(parse(&args(&["--wait", "7200"])).is_err());
    }

    #[test]
    fn the_start_is_a_depth_from_one_to_thirty() {
        assert_eq!(parse(&args(&["--start", "20"])).unwrap().start, Some(20));
        assert!(parse(&args(&["--start", "0"])).is_err());
        assert!(parse(&args(&["--start", "31"])).is_err());
    }
}
