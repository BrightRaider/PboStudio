//! The guest against a stand-in helper on a real pipe (gate G7, without the hardware): the guest drives a run through the
//! helper, a host that goes away mid-run leaves the run going, and a new guest finds it again.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use pbo_app::wire::parse_flat;
use pbo_embed_example::{Guest, HelperBackend};
use pbo_os::pipe::{self, Handler};

/// A helper that walks through cores on a clock, the way the real one reports a run.
struct Sim {
    started: Option<Instant>,
    cores: Vec<u32>,
    step: Duration,
    stopped: bool,
}

struct FakeHelper(Arc<Mutex<Sim>>);

impl Handler for FakeHelper {
    fn handle(&mut self, message: &str) -> String {
        let pairs = parse_flat(message).unwrap_or_default();
        let get = |k: &str| {
            pairs
                .iter()
                .find(|(key, _)| key == k)
                .map(|(_, v)| v.clone())
        };
        let mut sim = self.0.lock().unwrap();
        match get("type").as_deref() {
            Some("start") => {
                if sim.started.is_some() && !sim.stopped {
                    return "{\"ok\":false,\"error\":\"a run is already going on\"}".into();
                }
                sim.cores = get("cores")
                    .unwrap_or_default()
                    .split(',')
                    .filter_map(|c| c.parse().ok())
                    .collect();
                sim.started = Some(Instant::now());
                sim.stopped = false;
                "{\"ok\":true}".into()
            }
            Some("stop") => {
                sim.stopped = true;
                "{\"ok\":true}".into()
            }
            Some("status") => {
                let (running, core, finished) = match sim.started {
                    None => (false, -1, "null".to_string()),
                    Some(t) => {
                        let idx = (t.elapsed().as_millis() / sim.step.as_millis()) as usize;
                        if sim.stopped {
                            (false, -1, "null".to_string())
                        } else if idx >= sim.cores.len() {
                            (false, -1, "0".to_string())
                        } else {
                            (true, i64::from(sim.cores[idx]), "null".to_string())
                        }
                    }
                };
                format!(
                    "{{\"ok\":true,\"running\":{running},\"core\":{core},\"finished\":{finished}}}"
                )
            }
            _ => "{\"ok\":false,\"error\":\"unknown\"}".into(),
        }
    }
}

fn serve(name: &'static str, step_ms: u64) -> Arc<Mutex<Sim>> {
    let sim = Arc::new(Mutex::new(Sim {
        started: None,
        cores: Vec::new(),
        step: Duration::from_millis(step_ms),
        stopped: false,
    }));
    let handler: Arc<Mutex<dyn Handler>> = Arc::new(Mutex::new(FakeHelper(sim.clone())));
    std::thread::spawn(move || {
        let _ = pipe::serve(name, handler);
    });
    for _ in 0..100 {
        if pbo_embed_example::helper_call(name, "{\"type\":\"status\"}").is_ok() {
            return sim;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("the stand-in helper never came up");
}

fn guest(name: &str) -> Guest {
    let mut backend = HelperBackend::new(name);
    backend.poll = Duration::from_millis(30);
    Guest::new(Box::new(backend))
}

fn wait_for(g: &mut Guest, state: &str) -> String {
    let deadline = Instant::now() + Duration::from_secs(6);
    loop {
        let s = g.handle("{\"type\":\"status\"}");
        if s.contains(&format!("\"state\":\"{state}\"")) {
            return s;
        }
        assert!(
            Instant::now() < deadline,
            "never reached {state}; last: {s}"
        );
        std::thread::sleep(Duration::from_millis(15));
    }
}

#[test]
fn a_run_is_driven_through_the_helper() {
    serve("g7-a", 120);
    let mut g = guest("g7-a");
    g.handle("{\"type\":\"plan.set\",\"key\":\"cores\",\"value\":\"0,1,2\"}");
    assert_eq!(g.handle("{\"type\":\"start\"}"), "{\"ok\":true}");
    let running = wait_for(&mut g, "Running");
    assert!(running.contains("\"line\":\"Testing core"), "{running}");
    wait_for(&mut g, "Finished");
}

#[test]
fn a_host_that_goes_away_mid_run_leaves_the_run_going_and_the_next_guest_finds_it() {
    let sim = serve("g7-b", 150);
    {
        let mut first = guest("g7-b");
        first.handle("{\"type\":\"plan.set\",\"key\":\"cores\",\"value\":\"0,1,2,3\"}");
        first.handle("{\"type\":\"start\"}");
        wait_for(&mut first, "Running");
        // The host ends here: the guest is destroyed in the middle of the run.
    }
    assert!(
        !sim.lock().unwrap().stopped,
        "going away must not stop the run"
    );

    let mut second = guest("g7-b");
    assert!(
        second.attach(),
        "the new guest should find the run that is still going"
    );
    wait_for(&mut second, "Running");
    wait_for(&mut second, "Finished");
}

#[test]
fn stopping_goes_through_the_helper_and_ends_halted() {
    let sim = serve("g7-c", 300);
    let mut g = guest("g7-c");
    g.handle("{\"type\":\"plan.set\",\"key\":\"cores\",\"value\":\"0,1,2,3\"}");
    g.handle("{\"type\":\"start\"}");
    wait_for(&mut g, "Running");
    assert_eq!(g.handle("{\"type\":\"stop\"}"), "{\"ok\":true}");
    assert!(sim.lock().unwrap().stopped);
    wait_for(&mut g, "Halted");
}

#[test]
fn without_a_helper_a_start_is_an_answer_not_a_crash() {
    let mut g = guest("g7-nobody");
    let answer = g.handle("{\"type\":\"start\"}");
    assert!(
        answer.contains("\"ok\":false") && answer.contains("no helper"),
        "{answer}"
    );
    assert!(g
        .handle("{\"type\":\"status\"}")
        .contains("\"state\":\"Halted\""));
    assert!(!g.attach());
}
