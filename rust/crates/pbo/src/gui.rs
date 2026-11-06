//! The window (ADR-0009). It decides nothing: it reads the plan and the run state, draws them, and turns what the
//! person does into commands. The run itself is `commands::run_with` and `commands::tune_with` on a worker thread,
//! the same code the command line runs; they report back through a channel.

use std::collections::{BTreeMap, VecDeque};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use eframe::egui;
use pbo_app::plan::{Engine, Plan, Search};
use pbo_app::text::{self, Key, Lang};
use pbo_core::auto::Intensity;
use pbo_core::knowledge::Knowledge;
use pbo_core::runstate::{self, Command, Controls, HaltReason, State};
use pbo_os as os;
use pbo_ui::{self as ui, ChipKind};

use crate::commands::{self, Options};
use crate::helperlink::{Link, Remote, WINDOW_PIPE};
use crate::machine::{Machine, Sample};
use crate::session::Event;
use crate::store::Store;
use crate::stress;

mod bar;
mod cores;
mod header;
mod layout;
mod pages;
mod panel;
mod preview;
pub use preview::preview;
use preview::Shot;

const HISTORY: usize = 240;
const IDLE_REFRESH: Duration = Duration::from_secs(2);

struct Worker {
    rx: Receiver<Event>,
}

/// What the window shows about the processor. Separate from the hardware handle so that a state can be drawn without
/// any hardware (the previews).
#[derive(Clone)]
struct MachineInfo {
    brand: String,
    family_name: String,
    verified: bool,
    cores: usize,
    deepest: i32,
    shallowest: i32,
    /// Where the search begins for a core that holds nothing yet (the family's suggestion).
    suggested_start: i32,
}

struct App {
    dark: bool,
    lang: Lang,
    tab: usize,
    plan: Plan,
    /// No core is chosen. The plan says "no cores" by an empty list, which means all of them, so the case where the person
    /// has switched every core off is kept here, and nothing can be started while it holds.
    none_selected: bool,
    /// Step, safety and start of Auto are shown (opened with a button, session only).
    auto_more: bool,
    /// How thorough Auto is: remembered with the plan.
    intensity: Intensity,
    /// The steps of the search Auto makes, when the person chose their own; none: those of the intensity.
    auto_step: Option<u32>,
    /// The campaign phase the next run is part of (0: none), set by Auto, taken by the start.
    next_phase: u32,
    /// The campaign phase of the run that is going on (0: none): what the run view calls its step.
    run_phase: u32,
    /// Which page of the plan builder is open: 0 what to test, 1 how to run.
    adv_page: usize,
    /// A run ended and the window should call for attention in the taskbar: true when it ended badly.
    attention: Option<bool>,
    machine: Option<Machine>,
    /// The helper with the rights, when this window runs without them (mode B).
    link: Option<Link>,
    /// This window started the helper, so it ends it again when nothing is running.
    started_helper: bool,
    /// Asked for the helper and waiting for it to answer.
    waiting_for_helper: Option<Instant>,
    quit: std::sync::Arc<std::sync::atomic::AtomicBool>,
    info: Option<MachineInfo>,
    machine_error: Option<String>,
    engine_found: bool,
    held: Vec<i32>,
    knowledge: Knowledge,
    state: State,
    worker: Option<Worker>,
    log: Vec<String>,
    temp: VecDeque<f32>,
    ppt: VecDeque<f32>,
    vcore: VecDeque<f32>,
    last_sample: Sample,
    last_refresh: Instant,
    began: bool,
    test_began: Instant,
    test_seconds: u64,
    current: Option<(usize, i32)>,
    toast: Option<(String, Instant)>,
    selftest: Option<Result<(), String>>,
    readiness: Option<Vec<pbo_core::readiness::Check>>,
    /// A setup step that is running: the name of what it installs and the channel its result comes back on.
    setup_busy: Option<Receiver<i32>>,
    /// Putting the proven values back after a cut-off run.
    restore_busy: Option<Receiver<i32>>,
    /// The last run did not end (a state file is there).
    interrupted: bool,
    /// Values typed into the core rows, not yet written.
    edit: std::collections::BTreeMap<usize, i32>,
    /// Whether the logon task is installed; asked at start and after a change, not every frame.
    logon_task: bool,
    /// The logon task carries Auto on after a crash.
    logon_continue: bool,
    store: Store,
    shot: Option<Shot>,
    /// What was last written to `window.json`, to write again only when something changed.
    window_saved: String,
    first_frame: bool,
    /// The drawer at the bottom: 0 the telemetry, 1 the log.
    drawer: Option<u8>,
    show_bios: bool,
    show_setup: bool,
    profile_pick: usize,
    /// What the cores held when the window opened: after a restart these are the BIOS values.
    start_values: Vec<i32>,
    log_tail: Vec<String>,
}

fn push(history: &mut VecDeque<f32>, value: Option<f32>) {
    if let Some(v) = value {
        if history.len() == HISTORY {
            history.pop_front();
        }
        history.push_back(v);
    }
}

impl App {
    pub(crate) fn new(cc: &eframe::CreationContext) -> App {
        let dark = true;
        ui::apply(&cc.egui_ctx, dark);
        let elevated = crate::machine::is_elevated();
        let (machine, machine_error) = if elevated {
            match Machine::open() {
                Ok(m) => (Some(m), None),
                Err(e) => (None, Some(e)),
            }
        } else {
            (None, None)
        };
        let info = machine.as_ref().map(|m| MachineInfo {
            brand: m.brand.clone(),
            family_name: m.family.name.to_string(),
            verified: m.family.verified,
            cores: m.cores,
            deepest: m.allowed_range().0,
            shallowest: m.allowed_range().1,
            suggested_start: pbo_core::families::suggested_start(&m.family, m.x3d),
        });
        // Without rights, a helper that is already running (started earlier, or by a run that outlived its window) is
        // picked up; nothing is started without the person asking.
        let link = if elevated {
            None
        } else {
            let l = Link::new(WINDOW_PIPE);
            l.status().ok().map(|_| l)
        };
        let store = Store::beside_program();
        let (knowledge, _) = store.load_knowledge();
        let mut app = App {
            dark,
            lang: if os::ui_language_is_german() {
                Lang::De
            } else {
                Lang::En
            },
            tab: 0,
            plan: Plan::recommended(),
            none_selected: false,
            auto_more: false,
            // Chosen beforehand: the quick run is the efficient one (the proof at the end is the same length in every run of
            // it, and short tests of 2 to 3 minutes are what find a core that does not hold).
            intensity: Intensity::Fast,
            auto_step: None,
            next_phase: 0,
            run_phase: 0,
            adv_page: 0,
            attention: None,
            machine,
            link,
            started_helper: false,
            waiting_for_helper: None,
            quit: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            info,
            machine_error,
            engine_found: stress::find_prime95().is_some(),
            held: Vec::new(),
            knowledge,
            state: State::Ready,
            worker: None,
            log: Vec::new(),
            temp: VecDeque::new(),
            ppt: VecDeque::new(),
            vcore: VecDeque::new(),
            last_sample: Sample::default(),
            last_refresh: Instant::now() - IDLE_REFRESH,
            began: false,
            test_began: Instant::now(),
            test_seconds: 0,
            current: None,
            toast: None,
            selftest: None,
            readiness: None,
            setup_busy: None,
            restore_busy: None,
            interrupted: false,
            edit: std::collections::BTreeMap::new(),
            logon_task: false,
            logon_continue: false,
            store,
            shot: None,
            window_saved: String::new(),
            first_frame: true,
            drawer: None,
            show_bios: false,
            show_setup: false,
            profile_pick: 0,
            start_values: Vec::new(),
            log_tail: Vec::new(),
        };
        // What the window remembered: the theme, the language and the plan. Saved on every change, not at the end, because
        // a crash is the normal case here.
        let saved = app.store.load_window();
        if let Some(d) = saved.dark {
            app.dark = d;
            ui::apply(&cc.egui_ctx, d);
        }
        match saved.language.as_deref() {
            Some("de") => app.lang = Lang::De,
            Some("en") => app.lang = Lang::En,
            _ => {}
        }
        if let Some(p) = saved.plan.and_then(|t| Plan::from_toml(&t).ok()) {
            app.plan = p;
        }
        if let Some(i) = saved.intensity.as_deref().and_then(Intensity::from_id) {
            app.intensity = i;
        }
        app.auto_step = saved
            .auto_step
            .filter(|n| (1..=pbo_core::tuner::MAX_STEP as u32).contains(n));
        app.window_saved = app.window_state();
        app.interrupted = app.store.read_run_state().is_some();
        app.logon_task = crate::ready::logon_task_installed();
        app.logon_continue = crate::ready::logon_task_continues();
        app.refresh();
        app
    }

    fn t(&self, key: Key) -> &'static str {
        text::tr(self.lang, key)
    }

    /// Reads the held values and one sample from the processor (only when no run is using it).
    /// Takes in what the helper says about the processor.
    fn apply_remote(&mut self, r: &Remote) {
        self.info = Some(MachineInfo {
            brand: r.cpu.clone(),
            family_name: r.family.clone(),
            verified: r.verified,
            cores: r.cores,
            deepest: r.deepest,
            shallowest: r.shallowest,
            suggested_start: r.suggested_start,
        });
        self.held = r.margins.clone();
    }

    fn refresh(&mut self) {
        self.last_refresh = Instant::now();
        if let Some(link) = self.link.clone() {
            match link.status() {
                Ok(r) => self.apply_remote(&r),
                Err(_) => {
                    // The helper is gone: back to asking for one.
                    self.link = None;
                    self.info = None;
                    return;
                }
            }
            if let Ok(s) = link.sample() {
                push(&mut self.temp, s.temperature);
                push(&mut self.ppt, s.ppt_watts);
                push(&mut self.vcore, s.core_volts.map(|v| v as f32));
                self.last_sample = s;
            }
        } else if let Some(m) = &self.machine {
            self.held = m.read_margins().unwrap_or_default();
            let s = m.sample();
            push(&mut self.temp, s.temperature);
            push(&mut self.ppt, s.ppt_watts);
            push(&mut self.vcore, s.core_volts.map(|v| v as f32));
            self.last_sample = s;
        }
        if self.start_values.is_empty() && !self.held.is_empty() {
            self.start_values = self.held.clone();
        }
        if self.drawer == Some(1) {
            self.refresh_log_tail();
        }
        self.knowledge = self.store.load_knowledge().0;
    }

    fn command(&mut self, c: Command) {
        match runstate::apply(&self.state, c) {
            Ok(next) => {
                self.state = next;
                // A run that is over (however it ended) is nothing to carry on after a crash.
                if matches!(
                    self.state,
                    State::Ready | State::Finished | State::Halted(_)
                ) && !self.is_preview()
                {
                    let _ = std::fs::remove_file(self.store.root.join("auto_last.txt"));
                }
            }
            Err(e) => self
                .log
                .push(format!("(ignored: {:?} in {:?})", e.command, e.from)),
        }
    }

    fn start(&mut self) {
        let plan = self.plan.clone();
        self.start_with(plan);
    }

    /// Whether this window is a preview with made-up numbers: what a scripted click does must stay inside the picture. Nothing
    /// that writes a file, a task or a value, installs something or starts a run may happen then (the scripted real runs
    /// are the one exception and say so themselves).
    pub(super) fn is_preview(&self) -> bool {
        self.shot.as_ref().is_some_and(|s| !s.is_live_run())
    }

    /// Starts a run of this plan, the same way whichever page asked for it.
    fn start_with(&mut self, plan: Plan) {
        // Nothing is chosen: a plan with no cores would mean all of them, which is not what the person asked for.
        if !self.has_cores() {
            return;
        }
        // A preview has made-up numbers and no hardware: a scripted click on a start button must not start a real run.
        if self.shot.as_ref().is_some_and(|s| !s.is_live_run()) {
            return;
        }
        self.run_phase = std::mem::take(&mut self.next_phase);
        let o = Options::from_plan(&plan);
        let tune = plan.search != Search::Off;
        self.command(Command::Start);
        if self.state != State::Preparing {
            return;
        }
        os::reset_cancel();
        self.log.clear();
        self.began = false;
        self.current = None;
        let (tx, rx) = mpsc::channel();
        if let Some(link) = self.link.clone() {
            // Mode B: the helper runs it; this window watches. The helper keeps going if the window closes.
            if let Err(e) = link.start(&plan) {
                self.log.push(e);
                self.command(Command::Abort(HaltReason::Inconclusive));
                return;
            }
            self.quit.store(false, std::sync::atomic::Ordering::SeqCst);
            link.bridge(tx, self.quit.clone());
            self.worker = Some(Worker { rx });
            return;
        }
        self.worker = Some(Worker { rx });
        // The worker opens its own handles to the driver; the window keeps its own for reading.
        std::thread::spawn(move || {
            let code = if tune {
                commands::tune_with(&o, Some(tx.clone()))
            } else {
                commands::run_with(&o, Some(tx.clone()))
            };
            let _ = tx.send(Event::Finished(code));
        });
    }

    fn poll_worker(&mut self) {
        let mut events = Vec::new();
        if let Some(w) = &self.worker {
            while let Ok(e) = w.rx.try_recv() {
                events.push(e);
            }
        }
        for e in events {
            match e {
                Event::Line(l) => self.log.push(l),
                Event::Sample(sample) => {
                    push(&mut self.temp, sample.temperature);
                    push(&mut self.ppt, sample.ppt_watts);
                    push(&mut self.vcore, sample.core_volts.map(|v| v as f32));
                    self.last_sample = sample;
                }
                Event::Progress {
                    core,
                    margin,
                    seconds,
                } => {
                    self.current = Some((core, margin));
                    self.test_began = Instant::now();
                    self.test_seconds = seconds;
                    if self.began {
                        self.command(Command::Advance {
                            phase: self.run_phase,
                            core: core as u32,
                        });
                    } else {
                        self.began = true;
                        self.command(Command::Begin {
                            phase: self.run_phase,
                            core: core as u32,
                        });
                    }
                }
                Event::Finished(code) => {
                    self.worker = None;
                    self.attention = Some(code != 0);
                    match &self.state {
                        State::Stopping => self.command(Command::Done),
                        State::Running { .. } => {
                            let _ = code;
                            self.command(Command::Done)
                        }
                        State::Preparing => self.command(Command::Abort(HaltReason::Inconclusive)),
                        _ => {}
                    }
                    self.current = None;
                    self.refresh();
                }
            }
        }
    }

    /// While waiting for a helper that was just asked for: look for it, and when it answers, take it over, together
    /// with a run it may already be doing.
    fn meet_helper(&mut self) {
        let Some(since) = self.waiting_for_helper else {
            return;
        };
        if self.link.is_some() || since.elapsed() > Duration::from_secs(60) {
            self.waiting_for_helper = None;
            return;
        }
        if self.last_refresh.elapsed() < Duration::from_millis(500) {
            return;
        }
        self.last_refresh = Instant::now();
        let link = Link::new(WINDOW_PIPE);
        if let Ok(remote) = link.status() {
            self.waiting_for_helper = None;
            self.apply_remote(&remote);
            if remote.running {
                self.picked_up_run(&link);
            }
            self.link = Some(link);
        }
    }

    /// A run the helper is already doing (it outlived an earlier window): watch it.
    fn picked_up_run(&mut self, link: &Link) {
        self.command(Command::Start);
        if self.state != State::Preparing {
            return;
        }
        // Whose run it is, and so which phase of the campaign, is not known.
        self.run_phase = 0;
        self.began = false;
        let (tx, rx) = mpsc::channel();
        self.quit.store(false, std::sync::atomic::Ordering::SeqCst);
        link.bridge(tx, self.quit.clone());
        self.worker = Some(Worker { rx });
    }

    /// Takes in the result of a setup step that was running.
    fn poll_setup(&mut self) {
        let Some(rx) = &self.setup_busy else {
            return;
        };
        if let Ok(code) = rx.try_recv() {
            self.setup_busy = None;
            self.engine_found = stress::find_prime95().is_some();
            self.logon_task = crate::ready::logon_task_installed();
            self.logon_continue = crate::ready::logon_task_continues();
            let msg = if code == 0 {
                self.t(Key::SetupDone)
            } else {
                self.t(Key::SetupFailed)
            };
            self.say_toast(msg);
        }
    }

    fn poll_restore(&mut self) {
        let Some(rx) = &self.restore_busy else {
            return;
        };
        if let Ok(code) = rx.try_recv() {
            self.restore_busy = None;
            let msg = if code == 0 {
                self.interrupted = self.store.read_run_state().is_some();
                self.t(Key::Restored)
            } else {
                self.t(Key::SetupFailed)
            };
            self.say_toast(msg);
            self.refresh();
        }
    }

    fn say_toast(&mut self, text: &str) {
        self.toast = Some((text.to_string(), Instant::now()));
    }

    // ── Drawing ───────────────────────────────────────────────────────────

    /// What `window.json` holds, as one text, to compare.
    fn window_state(&self) -> String {
        format!(
            "{}|{}|{}|{:?}|{}",
            self.dark,
            if self.lang == Lang::De { "de" } else { "en" },
            self.intensity.id(),
            self.auto_step,
            self.plan.to_toml()
        )
    }

    fn save_window_if_changed(&mut self) {
        // The previews and the scripted runs leave the person's file alone.
        if self.shot.is_some() {
            return;
        }
        let state = self.window_state();
        if state != self.window_saved {
            let _ = self.store.save_window(&crate::store::WindowFile {
                dark: Some(self.dark),
                language: Some(if self.lang == Lang::De { "de" } else { "en" }.to_string()),
                plan: Some(self.plan.to_toml()),
                intensity: Some(self.intensity.id().to_string()),
                auto_step: self.auto_step,
            });
            self.window_saved = state;
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.first_frame {
            self.first_frame = false;
            // A window that is started from somewhere else may open behind the game; ask for the front.
            if self.shot.is_none() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }
        self.poll_worker();
        // A run that ended while the window is behind others: the taskbar says so.
        if let Some(bad) = self.attention.take() {
            if self.shot.is_none() {
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(if bad {
                    egui::UserAttentionType::Critical
                } else {
                    egui::UserAttentionType::Informational
                }));
            }
        }
        self.poll_setup();
        self.poll_restore();
        self.save_window_if_changed();
        self.take_shot(ctx);
        self.meet_helper();
        if self.worker.is_none()
            && (self.machine.is_some() || self.link.is_some())
            && self.last_refresh.elapsed() >= IDLE_REFRESH
        {
            self.refresh();
        }

        self.layout(ctx);

        // A short notice that goes away by itself.
        if let Some((text, at)) = &self.toast {
            if at.elapsed() < Duration::from_secs(3) {
                egui::Area::new("toast".into())
                    .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -16.0])
                    .show(ctx, |ui_| {
                        ui::card(ui_, self.dark, None, |ui_| {
                            ui_.label(text);
                        });
                    });
            } else {
                self.toast = None;
            }
        }

        ctx.request_repaint_after(Duration::from_millis(500));
    }

    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        self.scripted_input(raw_input);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // With a helper the run is the helper's: closing the window leaves it going, and a window that started the
        // helper ends it only when nothing is running.
        if let Some(link) = &self.link {
            self.quit.store(true, std::sync::atomic::Ordering::SeqCst);
            if self.started_helper && self.worker.is_none() {
                link.shutdown();
            }
            return;
        }
        // Closing in the middle of a run: stop the engine and put the values back before the process ends.
        if self.worker.is_some() {
            os::request_cancel();
            let deadline = Instant::now() + Duration::from_secs(10);
            while self.worker.is_some() && Instant::now() < deadline {
                self.poll_worker();
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

/// The window icon: the program's mark, drawn (`pbo-ui/src/icon.rs`); the same picture is built into the executable.
fn icon() -> egui::IconData {
    let size = 128usize;
    egui::IconData {
        rgba: ui::icon::rgba(size),
        width: size as u32,
        height: size as u32,
    }
}

pub fn run() -> i32 {
    os::hide_own_console();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("PboStudio")
            .with_icon(icon())
            .with_inner_size([1180.0, 780.0])
            .with_min_inner_size([980.0, 668.0]),
        ..Default::default()
    };
    match eframe::run_native(
        "PboStudio",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    ) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("the window could not be opened: {e}");
            1
        }
    }
}
