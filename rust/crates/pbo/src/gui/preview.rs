//! Previews: every state of the window, drawn with made-up numbers and no hardware (ADR-0009, point 6).
//!
//! `pbo gui-preview NAME FILE.png` opens the window out of sight, lets it settle, asks it for a screenshot of its own
//! buffer, writes the PNG and leaves. The pictures are the reference that a change to the look is compared with.

use super::*;

/// The names `pbo gui-preview` knows.
pub const PREVIEWS: [&str; 20] = [
    // The real constructor and a scripted short run through the window: needs administrator rights.
    "live-run",
    // The real constructor and Auto, one core, the fast intensity, phase after phase: needs rights, writes like Auto.
    "live-auto",
    // The real constructor, out of sight: reads this machine as the window would at start.
    "live",
    "interrupted",
    "ready",
    "tests",
    "advanced",
    "notchecked",
    "preparing",
    "running",
    "stopping",
    "result",
    "halted",
    "notelevated",
    "telemetry",
    "log",
    "setup",
    "bios",
    // Sixteen cores: the rows have to give way.
    "many",
    // The second page of the plan builder.
    "advanced-run",
];

fn curve(base: f32, swing: f32, n: usize, phase: f32) -> VecDeque<f32> {
    (0..n)
        .map(|i| {
            base + swing * ((i as f32 / 17.0) + phase).sin()
                + swing * 0.3 * ((i as f32 / 5.0) + phase).cos()
        })
        .collect()
}

pub(super) struct Shot {
    path: String,
    frames: u32,
    /// Starts a short verification run through the window itself and takes the picture in the middle of it.
    live_run: bool,
    /// Starts Auto through the window on one core at the fast intensity, one phase after the other, until nothing is left
    /// to do or the time is up, and takes a picture in the middle of a run and one at the end. Needs administrator rights
    /// or a helper, and it writes to the processor the way Auto does: only for an explicit check.
    live_auto: bool,
    /// What the Auto scenario has done so far.
    auto_starts: u32,
    auto_mid_requested: bool,
    auto_final_requested: bool,
    /// The file the picture that was asked for goes to.
    pending_path: Option<String>,
    /// The scenario ends when this picture is written.
    closing: bool,
    run_started: Option<Instant>,
    requested: bool,
    /// Where the scripted pointer clicks, in order (`click=X,Y` on the command line). The clicks go through egui's own
    /// input, so the window finds what is under the pointer the way it does for a hand: nothing is called directly.
    clicks: Vec<egui::Pos2>,
    /// How many frames the input hook has seen.
    input_frames: u32,
}

/// The frame a scripted click starts on, and how many frames one click takes (move, press, release, settle).
const FIRST_CLICK_FRAME: u32 = 6;
const FRAMES_PER_CLICK: u32 = 5;

impl Shot {
    /// Whether this is the scripted real run, the only preview that is allowed to start one.
    pub(super) fn is_live_run(&self) -> bool {
        self.live_run || self.live_auto
    }

    /// The frame the picture is taken on: after the last click has settled.
    fn shoot_at(&self) -> u32 {
        FIRST_CLICK_FRAME + FRAMES_PER_CLICK * self.clicks.len() as u32 + 6
    }
}

impl App {
    /// The Auto scenario, one frame of it. The window starts Auto the way a click on Start does; a phase that has ended
    /// is dismissed the way a click on Done does, and the next one is started, until Auto has nothing left that it
    /// recommends or 45 minutes are up. It says what happens on its standard output.
    fn drive_live_auto(&mut self, ctx: &egui::Context) {
        let Some(shot) = &self.shot else {
            return;
        };
        let (frames, starts, mid_done, final_done) = (
            shot.frames,
            shot.auto_starts,
            shot.auto_mid_requested,
            shot.auto_final_requested,
        );
        let began = shot.run_started.unwrap_or_else(Instant::now);
        let running = matches!(
            self.state,
            State::Preparing | State::Running { .. } | State::Stopping
        );
        let ended = matches!(self.state, State::Finished | State::Halted(_));

        if frames == 3 {
            self.plan.cores = vec![1];
            self.intensity = Intensity::Fast;
            println!("auto: starting run 1 on core 1, fast");
            self.start_auto();
            if let Some(s) = &mut self.shot {
                s.run_started = Some(Instant::now());
                s.auto_starts = 1;
            }
            return;
        }
        if began.elapsed() > Duration::from_secs(45 * 60) && !final_done {
            println!("auto: the 45 minutes are up, stopping");
            self.stop_run();
            if let Some(s) = &mut self.shot {
                s.auto_final_requested = true;
            }
            return;
        }
        // A picture in the middle of the first run.
        if running && !mid_done && starts >= 1 && began.elapsed() >= Duration::from_secs(40) {
            println!("auto: picture in the middle of the run");
            if let Some(s) = &mut self.shot {
                s.auto_mid_requested = true;
                s.pending_path = Some(s.path.replace(".png", "-running.png"));
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            return;
        }
        if ended && !final_done {
            println!("auto: run {starts} ended: {:?}", self.state);
            self.command(Command::Dismiss);
            self.refresh();
            let next = self.campaign_view();
            let more = starts < 2
                && next
                    .as_ref()
                    .is_some_and(|v| v.step.action != pbo_core::campaign::Action::OpenSetup);
            if more {
                println!(
                    "auto: starting run {}: stage {:?}",
                    starts + 1,
                    next.map(|v| v.step.stage)
                );
                self.start_auto();
                if let Some(s) = &mut self.shot {
                    s.auto_starts = starts + 1;
                }
            } else {
                println!("auto: nothing more to start, final picture");
                if let Some(s) = &mut self.shot {
                    s.auto_final_requested = true;
                    s.closing = true;
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            }
        }
    }

    /// Feeds the scripted pointer into the frame that is about to be drawn.
    pub(super) fn scripted_input(&mut self, raw_input: &mut egui::RawInput) {
        let Some(shot) = &mut self.shot else {
            return;
        };
        let frame = shot.input_frames;
        shot.input_frames += 1;
        if frame < FIRST_CLICK_FRAME {
            return;
        }
        let n = ((frame - FIRST_CLICK_FRAME) / FRAMES_PER_CLICK) as usize;
        let step = (frame - FIRST_CLICK_FRAME) % FRAMES_PER_CLICK;
        let Some(pos) = shot.clicks.get(n).copied() else {
            return;
        };
        let button = |pressed: bool| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        match step {
            0 => raw_input.events.push(egui::Event::PointerMoved(pos)),
            1 => raw_input.events.push(button(true)),
            2 => raw_input.events.push(button(false)),
            _ => {}
        }
    }

    /// An app in a given state with made-up numbers. Nothing is read from the processor or from disk.
    fn for_preview(cc: &eframe::CreationContext, name: &str, dark: bool, german: bool) -> App {
        ui::apply(&cc.egui_ctx, dark);
        let verified = name != "notchecked";
        let mut knowledge = Knowledge::new();
        for (core, best, bad) in [
            (0u32, Some(-20), None),
            (1, Some(-25), None),
            (4, Some(-28), Some(-29)),
            (5, Some(-30), None),
        ] {
            knowledge.insert(
                core,
                pbo_core::knowledge::CoreKnowledge {
                    best_passed: best,
                    worst_failed: bad,
                },
            );
        }
        // A sixteen-core processor shows how the rows give way; everything else is the eight cores of the test machine.
        let many = name == "many";
        let cores = if many { 16 } else { 8 };
        let held: Vec<i32> = if many {
            (0..16).map(|c| -30 + (c % 5) * 3).collect()
        } else {
            vec![-20, -25, -30, -30, -28, -30, -30, -30]
        };
        let clocks: Vec<Option<f32>> = (0..cores)
            .map(|c| Some(4290.0 + 37.0 * ((c * 7) % 11) as f32))
            .collect();
        let state = match name {
            "preparing" => State::Preparing,
            "running" => State::Running { phase: 1, core: 4 },
            "stopping" => State::Stopping,
            "result" => State::Finished,
            "halted" => State::Halted(HaltReason::StoppedByPerson),
            _ => State::Ready,
        };
        let mut plan = Plan::recommended();
        if name == "advanced" || name == "advanced-run" {
            plan.cores = vec![2, 4, 5];
            plan.seconds_per_test = 120;
            plan.search = Search::Fine;
            plan.smt = true;
        }
        let log: Vec<String> = [
            "03:04:05 [1105030405] ---- start ----",
            "03:04:05 [1105030405] Tuning cores [4]: 60 s per test, coarse steps, headroom 3 step(s), range -30 to 0.",
            "03:04:06 [1105030405] core 4: Prime95 on logical [8] at -28 for 60 s",
            "03:04:11 [1105030405] telemetry core 4: temp 77.0C ppt 63.0W tdc 28.7A edc 112.4A vcore 1.212V",
            "03:05:06 [1105030405] core 4: engine used 99 % of the core",
            "03:05:06 [1105030405] core  4: held at -28 (3 full sweeps)",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        App {
            dark,
            none_selected: false,
            auto_more: false,
            auto_step: None,
            lang: if german { Lang::De } else { Lang::En },
            tab: match name {
                "tests" => 1,
                "advanced" | "advanced-run" => 2,
                _ => 0,
            },
            drawer: match name {
                "telemetry" => Some(0),
                "log" => Some(1),
                _ => None,
            },
            show_bios: name == "bios",
            show_setup: name == "setup",
            intensity: Intensity::Fast,
            next_phase: 0,
            run_phase: 1,
            adv_page: match name {
                "advanced" => 0,
                "advanced-run" => 1,
                _ => 0,
            },
            attention: None,
            profile_pick: if name == "tests" { 3 } else { 0 },
            start_values: vec![-20; cores],
            log_tail: vec![
                "03:04:05 [1105030405] ---- start ----".to_string(),
                "03:04:05 [1105030405] Testing cores [4] at the values they hold, 60 s each, with Prime95.".to_string(),
                "03:04:06 [1105030405] core 4: Prime95 on logical [8] at -28 for 60 s".to_string(),
                "03:04:11 [1105030405] telemetry core 4: temp 77.0C ppt 63.0W tdc 28.7A edc 112.4A vcore 1.212V".to_string(),
                "03:05:06 [1105030405] core 4: engine used 99 % of the core".to_string(),
                "03:05:06 [1105030405] core  4: held at -28 (3 full sweeps)".to_string(),
            ],
            plan,
            machine: None,
            link: None,
            started_helper: false,
            waiting_for_helper: None,
            quit: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            info: if name == "notelevated" {
                None
            } else {
                Some(MachineInfo {
                    brand: if many {
                        "AMD Ryzen 9 7950X 16-Core Processor".into()
                    } else {
                        "AMD Ryzen 7 5800X3D 8-Core Processor".into()
                    },
                    family_name: if many {
                        "Zen 4 (Raphael)".to_string()
                    } else {
                        "Zen 3 (Vermeer)".to_string()
                    },
                    verified,
                    cores,
                    deepest: -30,
                    shallowest: 0,
                    suggested_start: if many { -15 } else { -20 },
                })
            },
            machine_error: None,
            engine_found: true,
            held,
            knowledge,
            state,
            worker: None,
            log,
            temp: curve(74.0, 3.0, 120, 0.0),
            ppt: curve(62.0, 6.0, 120, 1.0),
            vcore: curve(1.212, 0.008, 120, 2.0),
            last_sample: Sample {
                temperature: Some(77.0),
                ppt_watts: Some(63.0),
                tdc_amps: Some(28.7),
                edc_amps: Some(112.4),
                temp_limit: Some(90.0),
                ppt_limit: Some(142.0),
                tdc_limit: Some(95.0),
                edc_limit: Some(130.0),
                clock_mhz: Some(4550.0),
                core_clocks: clocks,
                core_volts: Some(1.212),
                core_vids: vec![Some(1.212); cores],
            },
            last_refresh: Instant::now(),
            began: true,
            test_began: Instant::now() - Duration::from_secs(22),
            test_seconds: 60,
            current: Some((4, -28)),
            toast: None,
            selftest: if name == "setup" { Some(Ok(())) } else { None },
            setup_busy: None,
            restore_busy: None,
            interrupted: name == "interrupted",
            edit: std::collections::BTreeMap::new(),
            logon_task: false,
            readiness: if name == "advanced" {
                use pbo_core::readiness::{Check, Item, Level};
                Some(vec![
                    Check {
                        item: Item::LogonTask,
                        level: Level::Warn,
                    },
                    Check {
                        item: Item::AutoLogin,
                        level: Level::Warn,
                    },
                    Check {
                        item: Item::Sleep,
                        level: Level::Warn,
                    },
                    Check {
                        item: Item::Disk,
                        level: Level::Ok,
                    },
                    Check {
                        item: Item::Temperature,
                        level: Level::Unknown,
                    },
                ])
            } else {
                None
            },
            store: Store::beside_program(),
            shot: None,
            window_saved: String::new(),
            first_frame: false,
        }
    }

    /// Drives the screenshot: a few frames so fonts and layout settle, then the request, then the file.
    pub(super) fn take_shot(&mut self, ctx: &egui::Context) {
        let Some(shot) = &mut self.shot else {
            return;
        };
        shot.frames += 1;
        if shot.live_auto {
            self.drive_live_auto(ctx);
        } else if shot.live_run {
            // The scripted run: one core, a few seconds, nothing written to the processor. The window starts it the way a
            // click on Start does, so the worker, the channel and the state machine are the real ones.
            if shot.frames == 3 {
                shot.run_started = Some(Instant::now());
                self.plan = Plan {
                    cores: vec![4],
                    seconds_per_test: 12,
                    search: Search::Off,
                    ..Plan::recommended()
                };
                self.start();
            }
            let Some(shot) = &mut self.shot else {
                return;
            };
            let running = matches!(self.state, State::Running { .. });
            let due = running
                && shot
                    .run_started
                    .is_some_and(|t| t.elapsed() >= Duration::from_secs(7));
            if due && !shot.requested {
                shot.requested = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            }
        } else if shot.frames == shot.shoot_at() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
        }
        let Some(shot) = &mut self.shot else {
            return;
        };
        let image = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = image {
            let mut rgba = Vec::with_capacity(image.pixels.len() * 4);
            for p in &image.pixels {
                rgba.extend_from_slice(&[p.r(), p.g(), p.b(), p.a()]);
            }
            let png = ui::png::encode(image.size[0] as u32, image.size[1] as u32, &rgba);
            let path = shot
                .pending_path
                .take()
                .unwrap_or_else(|| shot.path.clone());
            if let Err(e) = std::fs::write(&path, png) {
                eprintln!("could not write {path}: {e}");
            }
            if !shot.live_auto || shot.closing {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        } else if shot.live_auto {
            // The scenario sets its own limit.
        } else if (!shot.live_run && shot.frames > shot.shoot_at() + 120)
            || shot
                .run_started
                .is_some_and(|t| t.elapsed() > Duration::from_secs(60))
        {
            eprintln!("no screenshot came back");
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint();
    }
}

/// `pbo gui-preview NAME FILE.png [light] [de]`.
pub fn preview(args: &[String]) -> i32 {
    let (Some(name), Some(out)) = (args.first(), args.get(1)) else {
        eprintln!(
            "usage: pbo gui-preview NAME FILE.png [light] [de] [size=WxH] [click=X,Y ...]\n  names: {}",
            PREVIEWS.join(" ")
        );
        return 2;
    };
    if !PREVIEWS.contains(&name.as_str()) {
        eprintln!("unknown state '{name}'; names: {}", PREVIEWS.join(" "));
        return 2;
    }
    let dark = !args.iter().any(|a| a == "light");
    let german = args.iter().any(|a| a == "de");
    // `size=980x640` shows the smallest window.
    let (width, height) = args
        .iter()
        .find_map(|a| a.strip_prefix("size="))
        .and_then(|s| s.split_once('x'))
        .and_then(|(w, h)| Some((w.parse::<f32>().ok()?, h.parse::<f32>().ok()?)))
        .unwrap_or((1180.0, 780.0));
    // `click=X,Y` (any number): the pointer clicks there, one after the other, before the picture is taken.
    let clicks: Vec<egui::Pos2> = args
        .iter()
        .filter_map(|a| a.strip_prefix("click="))
        .filter_map(|s| s.split_once(','))
        .filter_map(|(x, y)| Some(egui::Pos2::new(x.parse().ok()?, y.parse().ok()?)))
        .collect();
    let (name, out) = (name.clone(), out.clone());
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("PboStudio preview")
            .with_inner_size([width, height])
            // Out of sight: the picture is taken from the window's own buffer, not from the screen.
            .with_position([-4000.0, -4000.0])
            .with_taskbar(false),
        ..Default::default()
    };
    let result = eframe::run_native(
        "PboStudio preview",
        options,
        Box::new(move |cc| {
            let mut app = if name == "live" || name == "live-run" || name == "live-auto" {
                App::new(cc)
            } else {
                App::for_preview(cc, &name, dark, german)
            };
            let clicks = clicks.clone();
            app.shot = Some(Shot {
                path: out.clone(),
                frames: 0,
                live_run: name == "live-run",
                live_auto: name == "live-auto",
                auto_starts: 0,
                auto_mid_requested: false,
                auto_final_requested: false,
                pending_path: None,
                closing: false,
                run_started: None,
                requested: false,
                clicks,
                input_frames: 0,
            });
            Ok(Box::new(app))
        }),
    );
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("preview failed: {e}");
            1
        }
    }
}
