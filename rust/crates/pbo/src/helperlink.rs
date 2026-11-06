//! The window's side of the pipe to the helper (ADR-0014, mode B): the window runs without rights, the helper has them.
//!
//! The window never touches the processor in this mode. It asks the helper for the held values and for readings, starts
//! and stops runs through it, and turns what the helper reports about a run into the same events a run in its own process
//! would send, so the rest of the window does not know which mode it is in.

use std::sync::mpsc::Sender;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use pbo_app::plan::Plan;
use pbo_app::wire::{escape, parse_flat, parse_string_array};

use crate::machine::Sample;
use crate::session::Event;

/// The pipe name the window uses for its own helper.
pub const WINDOW_PIPE: &str = "window";

#[derive(Clone)]
pub struct Link {
    pub pipe: String,
}

/// What the helper says about the processor and the run.
#[derive(Debug, Clone, Default)]
pub struct Remote {
    pub cpu: String,
    pub family: String,
    pub verified: bool,
    pub cores: usize,
    pub margins: Vec<i32>,
    pub deepest: i32,
    pub shallowest: i32,
    /// Where the search begins for a core that holds nothing yet.
    pub suggested_start: i32,
    pub running: bool,
    pub core: Option<u32>,
    pub margin: Option<i32>,
    pub seconds: Option<u64>,
    pub finished: Option<i32>,
}

fn get<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    pairs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

impl Link {
    pub fn new(pipe: &str) -> Link {
        Link {
            pipe: pipe.to_string(),
        }
    }

    pub fn call(&self, message: &str) -> Result<String, String> {
        pbo_os::pipe::call(&self.pipe, message)
    }

    fn call_flat(&self, message: &str) -> Result<Vec<(String, String)>, String> {
        let answer = self.call(message)?;
        let pairs = parse_flat(&answer)?;
        if get(&pairs, "ok") == Some("true") {
            Ok(pairs)
        } else {
            Err(get(&pairs, "error")
                .unwrap_or("the helper refused")
                .to_string())
        }
    }

    pub fn status(&self) -> Result<Remote, String> {
        let p = self.call_flat("{\"type\":\"status\"}")?;
        let num = |k: &str| get(&p, k).and_then(|v| v.parse::<i64>().ok());
        Ok(Remote {
            cpu: get(&p, "cpu").unwrap_or("").to_string(),
            family: get(&p, "family").unwrap_or("").to_string(),
            verified: get(&p, "verified") == Some("true"),
            cores: num("cores").unwrap_or(0) as usize,
            margins: get(&p, "margins")
                .unwrap_or("")
                .split(',')
                .filter_map(|m| m.trim().parse().ok())
                .collect(),
            deepest: num("deepest").unwrap_or(-30) as i32,
            shallowest: num("shallowest").unwrap_or(0) as i32,
            suggested_start: num("start").unwrap_or(0) as i32,
            running: get(&p, "running") == Some("true"),
            core: num("core").filter(|c| *c >= 0).map(|c| c as u32),
            margin: num("margin").map(|m| m as i32),
            seconds: num("seconds").map(|s| s as u64),
            finished: num("finished").map(|c| c as i32),
        })
    }

    pub fn sample(&self) -> Result<Sample, String> {
        let p = self.call_flat("{\"type\":\"sample\"}")?;
        let f = |k: &str| get(&p, k).and_then(|v| v.parse::<f32>().ok());
        Ok(Sample {
            temperature: f("temp"),
            ppt_watts: f("ppt"),
            tdc_amps: f("tdc"),
            edc_amps: f("edc"),
            temp_limit: f("temp_limit"),
            ppt_limit: f("ppt_limit"),
            tdc_limit: f("tdc_limit"),
            edc_limit: f("edc_limit"),
            clock_mhz: f("clock"),
            core_clocks: get(&p, "clocks")
                .unwrap_or("")
                .split(',')
                .map(|c| c.trim().parse::<f32>().ok())
                .collect(),
            core_volts: f("vcore").map(f64::from),
            core_vids: Vec::new(),
        })
    }

    pub fn start(&self, plan: &Plan) -> Result<(), String> {
        let mut message = String::from("{\"type\":\"start\"");
        for (k, v) in plan.to_pairs() {
            message.push_str(&format!(",\"{k}\":\"{}\"", escape(&v)));
        }
        message.push('}');
        self.call_flat(&message).map(|_| ())
    }

    /// Asks the helper to put every core back to what it was proven to hold.
    pub fn resume(&self) -> bool {
        self.call_flat("{\"type\":\"resume\"}").is_ok()
    }

    /// Writes one core's value through the helper and returns what the processor holds afterwards.
    pub fn set(&self, core: usize, value: i32) -> Result<i32, String> {
        let p = self.call_flat(&format!(
            "{{\"type\":\"set\",\"core\":\"{core}\",\"value\":\"{value}\"}}"
        ))?;
        get(&p, "held")
            .and_then(|h| h.parse().ok())
            .ok_or_else(|| "the helper did not say what the core holds".to_string())
    }

    pub fn stop(&self) {
        let _ = self.call("{\"type\":\"stop\"}");
    }

    pub fn shutdown(&self) {
        let _ = self.call("{\"type\":\"shutdown\"}");
    }

    fn events(&self) -> (Vec<String>, Option<i32>) {
        let Ok(answer) = self.call("{\"type\":\"events\"}") else {
            return (Vec::new(), None);
        };
        let lines = parse_string_array(&answer, "lines");
        // "finished":null or a number, after the array; the flat parser cannot read past the array, so look for it.
        let finished = answer
            .split("\"finished\":")
            .nth(1)
            .map(|rest| rest.trim_end_matches('}').trim())
            .and_then(|v| v.parse::<i32>().ok());
        (lines, finished)
    }

    /// Watches the helper's run and sends what it sees as events, until the run is over. Does not stop the run when it
    /// is told to go away (`quit`): a window that closes mid-run leaves the run going.
    pub fn bridge(
        &self,
        tx: Sender<Event>,
        quit: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> JoinHandle<()> {
        let link = self.clone();
        std::thread::spawn(move || {
            let mut last: Option<(u32, i32)> = None;
            let mut next_sample = Instant::now();
            loop {
                if quit.load(std::sync::atomic::Ordering::SeqCst) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(400));
                let (lines, _) = link.events();
                for l in lines {
                    let _ = tx.send(Event::Line(l));
                }
                let Ok(status) = link.status() else {
                    let _ = tx.send(Event::Finished(1));
                    return;
                };
                if let (Some(core), Some(margin)) = (status.core, status.margin) {
                    if last != Some((core, margin)) {
                        last = Some((core, margin));
                        let _ = tx.send(Event::Progress {
                            core: core as usize,
                            margin,
                            seconds: status.seconds.unwrap_or(0),
                        });
                    }
                }
                if Instant::now() >= next_sample {
                    next_sample = Instant::now() + Duration::from_secs(2);
                    if let Ok(s) = link.sample() {
                        let _ = tx.send(Event::Sample(s));
                    }
                }
                if !status.running {
                    // One last drain: lines written in the final moments still count.
                    let (lines, _) = link.events();
                    for l in lines {
                        let _ = tx.send(Event::Line(l));
                    }
                    let _ = tx.send(Event::Finished(status.finished.unwrap_or(0)));
                    return;
                }
            }
        })
    }
}
