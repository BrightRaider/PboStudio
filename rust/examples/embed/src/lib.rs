//! A neutral example guest (ADR-0014): how `pbo-app` is built into a loadable module.
//!
//! The boundary is small and has four properties. It is a table of C functions. Strings are lent, never owned across
//! the boundary: the caller passes a message and a buffer for the answer, and nothing is allocated on one side and freed
//! on the other. New abilities are new messages, not new structures. A panic never crosses it. This module has its own
//! names (`pbo_...`); to put it into a particular host, replace the names and the message shapes with the host's and
//! leave the rest. Nothing here describes any host's interface.
//!
//! Messages are flat JSON objects with a `type`. Answers are flat JSON objects with `ok`.
//!
//! The backend that does the work is behind a trait. The example ships a stand-in that walks through a run on a thread,
//! so the loader test can show that starting, stopping and destroying behave; a real guest plugs in the client of the
//! helper process (ADR-0014, point 5) instead.

use std::ffi::{c_char, CStr};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use pbo_app::plan::Plan;
use pbo_app::text::Lang;
use pbo_app::wire::{self, escape, parse_flat};
use pbo_core::runstate::{self, Command, HaltReason, State};

/// Version of the table below. Changes only when the table itself changes.
pub const ABI_VERSION: u32 = 1;

/// What the backend tells the guest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendEvent {
    Began { core: u32 },
    Advanced { core: u32 },
    Done,
    Stopped,
}

/// The part that touches the processor. In a real guest this is the helper client; here it is a stand-in.
pub trait Backend: Send {
    fn start(&mut self, plan: &Plan, events: Sender<BackendEvent>) -> Result<(), String>;
    fn stop(&mut self);
    /// Picks up a run that is already going on somewhere else (the helper kept it going while this guest was gone).
    /// Returns whether there was one.
    fn attach(&mut self, _events: Sender<BackendEvent>) -> bool {
        false
    }
}

/// Walks through a run on a thread: began on core 0, advances through the planned cores, done. Joins its thread when
/// stopped or dropped, so destroying the guest leaves no thread behind.
pub struct StandIn {
    cancel: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    /// Pause between steps, so a test can stop a run in the middle.
    pub step: Duration,
}

impl StandIn {
    pub fn new(step: Duration) -> StandIn {
        StandIn {
            cancel: Arc::new(AtomicBool::new(false)),
            thread: None,
            step,
        }
    }
}

impl Backend for StandIn {
    fn start(&mut self, plan: &Plan, events: Sender<BackendEvent>) -> Result<(), String> {
        self.stop();
        self.cancel.store(false, Ordering::SeqCst);
        let cancel = self.cancel.clone();
        let cores = plan.cores_or_all(4);
        let step = self.step;
        self.thread = Some(std::thread::spawn(move || {
            for (i, core) in cores.iter().enumerate() {
                if cancel.load(Ordering::SeqCst) {
                    let _ = events.send(BackendEvent::Stopped);
                    return;
                }
                let core = *core as u32;
                let _ = events.send(if i == 0 {
                    BackendEvent::Began { core }
                } else {
                    BackendEvent::Advanced { core }
                });
                std::thread::sleep(step);
            }
            let _ = events.send(if cancel.load(Ordering::SeqCst) {
                BackendEvent::Stopped
            } else {
                BackendEvent::Done
            });
        }));
        Ok(())
    }

    fn stop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for StandIn {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The backend a real guest uses: it asks the helper process, which holds the rights and keeps the run going, over its
/// pipe. Written with the standard library only: the pipe is a file name. Dropping it detaches; it does not stop the
/// run, because a host that goes away in the middle of a run must not take the run with it.
pub struct HelperBackend {
    pipe: String,
    cancel: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    pub poll: Duration,
}

/// One request to the helper, one answer.
pub fn helper_call(pipe: &str, message: &str) -> Result<String, String> {
    use std::io::{Read, Write};
    let path = format!(r"\\.\pipe\pbo-helper-{pipe}");
    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|e| format!("no helper answers on '{pipe}': {e}"))?;
    f.write_all(message.as_bytes()).map_err(|e| e.to_string())?;
    let mut buffer = vec![0u8; 64 * 1024];
    let n = f.read(&mut buffer).map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&buffer[..n]).into_owned())
}

impl HelperBackend {
    pub fn new(pipe: &str) -> HelperBackend {
        HelperBackend {
            pipe: pipe.to_string(),
            cancel: Arc::new(AtomicBool::new(false)),
            thread: None,
            poll: Duration::from_millis(100),
        }
    }

    /// Watches the helper's run and turns what it sees into events, until the run is over or this backend detaches.
    fn watch(&mut self, events: Sender<BackendEvent>) {
        self.detach();
        self.cancel.store(false, Ordering::SeqCst);
        let (pipe, cancel, poll) = (self.pipe.clone(), self.cancel.clone(), self.poll);
        self.thread = Some(std::thread::spawn(move || {
            let mut last_core: Option<i64> = None;
            loop {
                if cancel.load(Ordering::SeqCst) {
                    return;
                }
                std::thread::sleep(poll);
                let Ok(answer) = helper_call(&pipe, "{\"type\":\"status\"}") else {
                    let _ = events.send(BackendEvent::Stopped);
                    return;
                };
                let Ok(pairs) = parse_flat(&answer) else {
                    continue;
                };
                let get = |k: &str| {
                    pairs
                        .iter()
                        .find(|(key, _)| key == k)
                        .map(|(_, v)| v.as_str())
                };
                let running = get("running") == Some("true");
                let core: i64 = get("core").and_then(|c| c.parse().ok()).unwrap_or(-1);
                if running {
                    if core >= 0 && last_core != Some(core) {
                        let core = core as u32;
                        let _ = events.send(if last_core.is_none() {
                            BackendEvent::Began { core }
                        } else {
                            BackendEvent::Advanced { core }
                        });
                        last_core = Some(core as i64);
                    }
                } else {
                    // Over. A code means it ran to its end; no code means it was stopped.
                    let finished = get("finished").is_some_and(|f| f != "null");
                    let _ = events.send(if finished && !cancel.load(Ordering::SeqCst) {
                        BackendEvent::Done
                    } else {
                        BackendEvent::Stopped
                    });
                    return;
                }
            }
        }));
    }

    /// Stops watching and waits for the watcher to end. The run itself is not touched.
    fn detach(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Backend for HelperBackend {
    fn start(&mut self, plan: &Plan, events: Sender<BackendEvent>) -> Result<(), String> {
        let mut message = String::from("{\"type\":\"start\"");
        for (k, v) in plan.to_pairs() {
            message.push_str(&format!(",\"{k}\":\"{}\"", escape(&v)));
        }
        message.push('}');
        let answer = helper_call(&self.pipe, &message)?;
        if !answer.contains("\"ok\":true") {
            return Err(answer);
        }
        self.watch(events);
        Ok(())
    }

    fn stop(&mut self) {
        let _ = helper_call(&self.pipe, "{\"type\":\"stop\"}");
        // Wait for the watcher to see the run end: by then the engine is gone and the values are put back.
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }

    fn attach(&mut self, events: Sender<BackendEvent>) -> bool {
        let running = helper_call(&self.pipe, "{\"type\":\"status\"}")
            .is_ok_and(|a| a.contains("\"running\":true"));
        if running {
            self.watch(events);
        }
        running
    }
}

impl Drop for HelperBackend {
    fn drop(&mut self) {
        self.detach();
    }
}

/// One loaded module: its plan, its state and its backend. No process-wide state.
pub struct Guest {
    plan: Plan,
    state: State,
    lang: Lang,
    backend: Box<dyn Backend>,
    rx: Receiver<BackendEvent>,
    tx: Sender<BackendEvent>,
    began: bool,
}

impl Guest {
    pub fn new(backend: Box<dyn Backend>) -> Guest {
        let (tx, rx) = mpsc::channel();
        Guest {
            plan: Plan::recommended(),
            state: State::Ready,
            lang: Lang::En,
            backend,
            rx,
            tx,
            began: false,
        }
    }

    /// Picks up a run the helper is already doing, for a guest that was started after the run began (the host
    /// restarted). Returns whether there was one.
    pub fn attach(&mut self) -> bool {
        if self.state != State::Ready || !self.backend.attach(self.tx.clone()) {
            return false;
        }
        self.apply(Command::Start);
        self.began = false;
        true
    }

    fn apply(&mut self, c: Command) {
        if let Ok(next) = runstate::apply(&self.state, c) {
            self.state = next;
        }
    }

    /// Takes in what the backend has reported since the last look.
    fn drain(&mut self) -> Vec<String> {
        let mut events = Vec::new();
        while let Ok(e) = self.rx.try_recv() {
            let name = match e {
                BackendEvent::Began { core } => {
                    self.began = true;
                    self.apply(Command::Begin { phase: 1, core });
                    "began"
                }
                BackendEvent::Advanced { core } => {
                    self.apply(Command::Advance { phase: 1, core });
                    "advanced"
                }
                BackendEvent::Done => {
                    if self.state == State::Preparing {
                        self.apply(Command::Abort(HaltReason::Inconclusive));
                    } else {
                        self.apply(Command::Done);
                    }
                    "done"
                }
                BackendEvent::Stopped => {
                    if self.state == State::Stopping {
                        self.apply(Command::Done);
                    }
                    "stopped"
                }
            };
            events.push(name.to_string());
        }
        events
    }

    fn status(&mut self) -> String {
        self.drain_into_state();
        let line = wire::status_line(self.lang, &self.state);
        let (phase, core) = wire::progress(&self.state).unwrap_or((0, 0));
        format!(
            "{{\"ok\":true,\"state\":\"{}\",\"line\":\"{}\",\"phase\":{phase},\"core\":{core}}}",
            wire::state_name(&self.state),
            escape(&line)
        )
    }

    fn drain_into_state(&mut self) {
        let _ = self.drain();
    }

    /// Answers one message. Never panics for an expected mistake; a wrong message is an `ok:false` answer.
    pub fn handle(&mut self, message: &str) -> String {
        let fail = |why: &str| format!("{{\"ok\":false,\"error\":\"{}\"}}", escape(why));
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
            Some("plan.get") => format!("{{\"ok\":true,\"plan\":{}}}", self.plan.to_json()),
            Some("plan.set") => {
                let (Some(key), Some(value)) = (get("key"), get("value")) else {
                    return fail("plan.set needs key and value");
                };
                if !runstate::Controls::of(&self.state).can_edit_plan {
                    return fail("the plan cannot be changed while a run is going on");
                }
                match self.plan.set(key, value) {
                    Ok(()) => "{\"ok\":true}".to_string(),
                    Err(e) => fail(&e),
                }
            }
            Some("lang") => {
                self.lang = match get("value") {
                    Some("de") => Lang::De,
                    Some("en") => Lang::En,
                    _ => return fail("lang is en or de"),
                };
                "{\"ok\":true}".to_string()
            }
            Some("start") => {
                if runstate::apply(&self.state, Command::Start).is_err() {
                    return fail("a run is already going on");
                }
                self.apply(Command::Start);
                self.began = false;
                if let Err(e) = self.backend.start(&self.plan, self.tx.clone()) {
                    self.apply(Command::Abort(HaltReason::Inconclusive));
                    return fail(&e);
                }
                "{\"ok\":true}".to_string()
            }
            Some("stop") => {
                if runstate::apply(&self.state, Command::Stop).is_err() {
                    return fail("nothing to stop");
                }
                self.apply(Command::Stop);
                self.backend.stop();
                self.drain_into_state();
                // The backend has ended; the stop has been carried out.
                if self.state == State::Stopping {
                    self.apply(Command::Done);
                }
                "{\"ok\":true}".to_string()
            }
            Some("dismiss") => match runstate::apply(&self.state, Command::Dismiss) {
                Ok(next) => {
                    self.state = next;
                    "{\"ok\":true}".to_string()
                }
                Err(_) => fail("nothing to dismiss"),
            },
            Some("status") => self.status(),
            Some("events") => {
                let list: Vec<String> = self.drain().iter().map(|e| format!("\"{e}\"")).collect();
                format!("{{\"ok\":true,\"events\":[{}]}}", list.join(","))
            }
            Some("test.panic") => panic!("a panic inside the adapter, on purpose"),
            Some(other) => fail(&format!("unknown message type '{other}'")),
            None => fail("a message needs a type"),
        }
    }
}

// ── The C boundary ────────────────────────────────────────────────────

/// The table a host reads. Nothing is allocated on one side and freed on the other.
#[repr(C)]
pub struct ModuleV1 {
    pub abi_version: u32,
    /// A new guest, or null.
    pub create: extern "C" fn() -> *mut Guest,
    /// Ends the guest: every thread it started is joined before this returns.
    pub destroy: extern "C" fn(*mut Guest),
    /// Sends one message (a NUL-terminated JSON object) and gets the answer in `out`, which has room for `capacity`
    /// bytes. Returns the length of the answer without the terminating NUL, or a negative code: -1 the arguments were
    /// not usable, -2 the buffer is too small (the answer is not written), -3 the adapter panicked (the guest stays
    /// usable and the host is untouched).
    pub send: extern "C" fn(*mut Guest, *const c_char, *mut c_char, usize) -> i32,
}

extern "C" fn create() -> *mut Guest {
    catch_unwind(|| {
        Box::into_raw(Box::new(Guest::new(Box::new(StandIn::new(
            Duration::from_millis(50),
        )))))
    })
    .unwrap_or(std::ptr::null_mut())
}

extern "C" fn destroy(guest: *mut Guest) {
    if !guest.is_null() {
        // Dropping the guest drops the backend, which joins its thread.
        let _ = catch_unwind(AssertUnwindSafe(|| drop(unsafe { Box::from_raw(guest) })));
    }
}

extern "C" fn send(
    guest: *mut Guest,
    message: *const c_char,
    out: *mut c_char,
    capacity: usize,
) -> i32 {
    if guest.is_null() || message.is_null() || out.is_null() || capacity == 0 {
        return -1;
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        let text = unsafe { CStr::from_ptr(message) }
            .to_string_lossy()
            .into_owned();
        unsafe { &mut *guest }.handle(&text)
    }));
    let Ok(answer) = result else {
        return -3;
    };
    let bytes = answer.as_bytes();
    if bytes.len() + 1 > capacity {
        return -2;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out.cast::<u8>(), bytes.len());
        *out.add(bytes.len()) = 0;
    }
    bytes.len() as i32
}

static MODULE: ModuleV1 = ModuleV1 {
    abi_version: ABI_VERSION,
    create,
    destroy,
    send,
};

/// The one exported symbol. A host looks it up, checks `abi_version`, and uses the table.
#[no_mangle]
pub extern "C" fn pbo_module_v1() -> *const ModuleV1 {
    &MODULE
}
