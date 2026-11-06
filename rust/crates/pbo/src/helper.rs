//! `pbo helper`: the part with the rights (ADR-0014, mode B; gate G7).
//!
//! The window, or a host's module, runs without administrator rights and talks to this process over a pipe. The
//! helper owns the processor handles, runs the tests, and keeps the run going when a front goes away: a front that
//! closes in the middle of a run does not stop it, and a new front finds the state again with `status` and `events`.
//! The messages are the same flat JSON the example guest speaks. The helper accepts only these messages, with limited
//! values; there are no paths and no commands in them.

use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};

use pbo_app::plan::{Plan, Search};
use pbo_app::wire::{escape, parse_flat};
use pbo_os as os;

use crate::commands::{self, Options};
use crate::machine::Machine;
use crate::session::Event;
use pbo_os::pipe::{self, Handler};

struct Run {
    rx: Receiver<Event>,
    /// Lines the run has produced and nobody has fetched yet.
    lines: Vec<String>,
    finished: Option<i32>,
    /// The core under test now, from the run's own progress.
    core: Option<u32>,
    margin: Option<i32>,
    seconds: Option<u64>,
}

struct Helper {
    machine: Machine,
    run: Option<Run>,
}

// SAFETY: the driver handles inside `Machine` are plain operating system handles that any thread may use; every access
// to the bus goes through the PCI lock, and the helper keeps the machine behind one mutex.
unsafe impl Send for Helper {}

impl Helper {
    fn running(&mut self) -> bool {
        self.pump();
        self.run.as_ref().is_some_and(|r| r.finished.is_none())
    }

    /// Moves what the run has reported into its buffer.
    fn pump(&mut self) {
        if let Some(run) = &mut self.run {
            while let Ok(e) = run.rx.try_recv() {
                match e {
                    Event::Line(l) => run.lines.push(l),
                    Event::Finished(code) => run.finished = Some(code),
                    Event::Progress {
                        core,
                        margin,
                        seconds,
                    } => {
                        run.core = Some(core as u32);
                        run.margin = Some(margin);
                        run.seconds = Some(seconds);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn fail(why: &str) -> String {
    format!("{{\"ok\":false,\"error\":\"{}\"}}", escape(why))
}

impl Handler for Helper {
    fn handle(&mut self, message: &str) -> String {
        let pairs = match parse_flat(message) {
            Ok(p) => p,
            Err(e) => return fail(&e),
        };
        let get = |k: &str| {
            pairs
                .iter()
                .find(|(key, _)| key == k)
                .map(|(_, v)| v.as_str())
        };
        match get("type") {
            Some("status") => {
                let running = self.running();
                let core = self.run.as_ref().and_then(|r| r.core).map_or(-1, i64::from);
                let margin = self
                    .run
                    .as_ref()
                    .and_then(|r| r.margin)
                    .map_or("null".to_string(), |m| m.to_string());
                let seconds = self
                    .run
                    .as_ref()
                    .and_then(|r| r.seconds)
                    .map_or("null".to_string(), |s| s.to_string());
                let (deepest, shallowest) = self.machine.allowed_range();
                let start =
                    pbo_core::families::suggested_start(&self.machine.family, self.machine.x3d);
                let finished = self
                    .run
                    .as_ref()
                    .and_then(|r| r.finished)
                    .map_or("null".to_string(), |c| c.to_string());
                let margins = self
                    .machine
                    .read_margins()
                    .map(|m| {
                        m.iter()
                            .map(|v| v.to_string())
                            .collect::<Vec<_>>()
                            .join(",")
                    })
                    .unwrap_or_default();
                format!(
                    "{{\"ok\":true,\"cpu\":\"{}\",\"family\":\"{}\",\"verified\":{},\"cores\":{},\"margins\":\"{margins}\",\"running\":{running},\"core\":{core},\"finished\":{finished},\"margin\":{margin},\"seconds\":{seconds},\"deepest\":{deepest},\"shallowest\":{shallowest},\"start\":{start}}}",
                    escape(&self.machine.brand),
                    escape(self.machine.family.name),
                    self.machine.family.verified,
                    self.machine.cores,
                )
            }
            Some("resume") => {
                if self.running() {
                    return fail("a run is going on");
                }
                let code = commands::resume(&[]);
                format!("{{\"ok\":{}}}", code == 0)
            }
            Some("sample") => {
                let s = self.machine.sample();
                let n = |v: Option<f32>| v.map_or("null".to_string(), |x| format!("{x:.2}"));
                let clocks = s
                    .core_clocks
                    .iter()
                    .map(|c| c.map_or(String::new(), |v| format!("{v:.0}")))
                    .collect::<Vec<_>>()
                    .join(",");
                let volts = s
                    .core_volts
                    .map_or("null".to_string(), |v| format!("{v:.4}"));
                format!(
                    "{{\"ok\":true,\"temp\":{},\"ppt\":{},\"tdc\":{},\"edc\":{},\"temp_limit\":{},\"ppt_limit\":{},\"tdc_limit\":{},\"edc_limit\":{},\"clock\":{},\"clocks\":\"{clocks}\",\"vcore\":{volts}}}",
                    n(s.temperature),
                    n(s.ppt_watts),
                    n(s.tdc_amps),
                    n(s.edc_amps),
                    n(s.temp_limit),
                    n(s.ppt_limit),
                    n(s.tdc_limit),
                    n(s.edc_limit),
                    n(s.clock_mhz),
                )
            }
            Some("set") => {
                if self.running() {
                    return fail("a run is going on; values are not changed under it");
                }
                let (Some(core), Some(value)) = (
                    get("core").and_then(|c| c.parse::<usize>().ok()),
                    get("value").and_then(|v| v.parse::<i32>().ok()),
                ) else {
                    return fail("set needs core and value as numbers");
                };
                if core >= self.machine.cores {
                    return fail("no such core");
                }
                // A family nobody has checked proves the write path on this core first.
                if pbo_core::families::needs_selftest(&self.machine.family) {
                    match self.machine.selftest(core) {
                        Ok((a, b)) if a == b => {}
                        Ok(_) | Err(_) => {
                            return fail("the write path did not prove itself on this core")
                        }
                    }
                }
                match self.machine.set_margin(core, value) {
                    Ok(held) => format!(
                        "{{\"ok\":true,\"held\":{held},\"stuck\":{}}}",
                        held == value
                    ),
                    Err(e) => fail(&e),
                }
            }
            Some("start") => {
                if self.running() {
                    return fail("a run is already going on");
                }
                let mut plan = Plan::recommended();
                for (k, v) in &pairs {
                    if k != "type" {
                        if let Err(e) = plan.set(k, v) {
                            return fail(&e);
                        }
                    }
                }
                let options = Options::from_plan(&plan);
                let tune = plan.search != Search::Off;
                let (tx, rx) = mpsc::channel();
                self.run = Some(Run {
                    rx,
                    lines: Vec::new(),
                    finished: None,
                    core: None,
                    margin: None,
                    seconds: None,
                });
                os::reset_cancel();
                std::thread::spawn(move || {
                    let code = if tune {
                        commands::tune_with(&options, Some(tx.clone()))
                    } else {
                        commands::run_with(&options, Some(tx.clone()))
                    };
                    let _ = tx.send(Event::Finished(code));
                });
                "{\"ok\":true}".to_string()
            }
            Some("stop") => {
                if !self.running() {
                    return fail("nothing is running");
                }
                os::request_cancel();
                "{\"ok\":true}".to_string()
            }
            Some("events") => {
                self.pump();
                let Some(run) = &mut self.run else {
                    return "{\"ok\":true,\"lines\":[],\"finished\":null}".to_string();
                };
                let lines: Vec<String> = run
                    .lines
                    .drain(..)
                    .map(|l| format!("\"{}\"", escape(&l)))
                    .collect();
                let finished = run.finished.map_or("null".to_string(), |c| c.to_string());
                format!(
                    "{{\"ok\":true,\"lines\":[{}],\"finished\":{finished}}}",
                    lines.join(",")
                )
            }
            Some("shutdown") => {
                if self.running() {
                    os::request_cancel();
                    // Give the run the moment it needs to stop its engine and put the values back.
                    for _ in 0..100 {
                        if !self.running() {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
                std::thread::spawn(|| {
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    std::process::exit(0);
                });
                "{\"ok\":true}".to_string()
            }
            Some(other) => fail(&format!("unknown message type '{other}'")),
            None => fail("a message needs a type"),
        }
    }
}

/// `pbo helper [--pipe NAME]`: serves until told to shut down.
pub fn run(args: &[String]) -> i32 {
    let name = args
        .windows(2)
        .find(|w| w[0] == "--pipe")
        .map_or("default", |w| w[1].as_str())
        .to_string();
    let machine = match Machine::open() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    println!("Helper serving on {}", pipe::pipe_path(&name));
    let helper: Arc<Mutex<dyn Handler>> = Arc::new(Mutex::new(Helper { machine, run: None }));
    match pipe::serve(&name, helper) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

/// `pbo helper-call NAME '{"type":"status"}'`: sends one message to a helper and prints the answer.
pub fn call(args: &[String]) -> i32 {
    let (Some(name), Some(message)) = (args.first(), args.get(1)) else {
        eprintln!("usage: pbo helper-call NAME MESSAGE   (for example: pbo helper-call default {{\"type\":\"status\"}})");
        return 2;
    };
    match pipe::call(name, message) {
        Ok(answer) => {
            println!("{answer}");
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

/// `pbo helper-watch NAME [FILE]`: starts the run in a plan file (or a short verification run of core 4 when there is
/// none) through a helper and prints what the window would see. It is the window's side of the pipe without the window.
pub fn watch(args: &[String]) -> i32 {
    use crate::helperlink::Link;
    use pbo_app::plan::{Plan, Search};
    let Some(name) = args.first() else {
        eprintln!("usage: pbo helper-watch NAME [FILE]");
        return 2;
    };
    let link = Link::new(name);
    let plan = match args.get(1) {
        Some(path) => match std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|t| Plan::from_toml(&t))
        {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{path}: {e}");
                return 2;
            }
        },
        None => Plan {
            cores: vec![4],
            seconds_per_test: 12,
            search: Search::Off,
            ..Plan::recommended()
        },
    };
    match link.status() {
        Ok(s) => println!(
            "{} ({}), {} cores, holds {:?}",
            s.cpu, s.family, s.cores, s.margins
        ),
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    }
    if let Err(e) = link.start(&plan) {
        eprintln!("not started: {e}");
        return 1;
    }
    let (tx, rx) = std::sync::mpsc::channel();
    let quit = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let handle = link.bridge(tx, quit);
    let mut code = 1;
    while let Ok(event) = rx.recv() {
        match event {
            crate::session::Event::Line(l) => println!("line: {l}"),
            crate::session::Event::Sample(s) => println!(
                "sample: {:?} C, {:?} W, {:?} V",
                s.temperature, s.ppt_watts, s.core_volts
            ),
            crate::session::Event::Progress {
                core,
                margin,
                seconds,
            } => {
                println!("progress: core {core} at {margin} for {seconds} s")
            }
            crate::session::Event::Finished(c) => {
                println!("finished: {c}");
                code = c;
                break;
            }
        }
    }
    let _ = handle.join();
    code
}
