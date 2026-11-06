//! What the window does on behalf of the person: the shortcuts, the checks of the setup, writing the values, the
//! campaign's next step as a plan, and the two small windows (the BIOS list, Driver & Engines). What is drawn is in
//! the files beside this one; the decisions are in `pbo-core` and the work is `commands`.

use super::*;
use pbo_core::auto;
use pbo_core::campaign::{self, Action};
use pbo_core::tuner::Mode;
use pbo_core::{estimate as est, profiles};
use pbo_ui::widgets as w;

/// What the Auto tab shows: the next step of the campaign and what is left of it, at each intensity.
pub(super) struct CampaignView {
    pub step: campaign::NextStep,
    pub phase: i32,
    /// What is left at the intensity that is chosen.
    pub estimate: est::Estimate,
    /// What is left at each intensity, worst case, in minutes: the numbers on the three tiles.
    pub by_intensity: Vec<(Intensity, f64)>,
    pub fresh: bool,
}

impl App {
    pub(super) fn shortcuts(&mut self, ctx: &egui::Context) {
        let (f5, ctrl_s, ctrl_l, escape) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::F5),
                i.modifiers.command && i.key_pressed(egui::Key::S),
                i.modifiers.command && i.key_pressed(egui::Key::L),
                i.key_pressed(egui::Key::Escape),
            )
        });
        // Escape closes the small windows.
        if escape {
            self.show_setup = false;
            self.show_bios = false;
        }
        if f5 {
            if Controls::of(&self.state).can_stop {
                self.stop_run();
            } else if Controls::of(&self.state).can_start && self.has_cores() {
                self.start_auto();
            }
        }
        if ctrl_s {
            self.apply_pending();
        }
        if ctrl_l {
            self.lang = if self.lang == Lang::De {
                Lang::En
            } else {
                Lang::De
            };
        }
    }

    /// The score of the setup: the driver, the test programs. Not the readiness list: that is about a night, this is
    /// about whether the program can work at all.
    pub(super) fn health(&self) -> (u32, Vec<&'static str>) {
        let mut score = 100u32;
        let mut missing = Vec::new();
        let driver_ok = self.link.is_some()
            || match pbo_smu::driver_status() {
                Ok(()) => true,
                // Access denied: the driver is there, this window just has no rights; the helper has them.
                Err(pbo_smu::Error::DriverUnavailable(5)) => true,
                Err(_) => false,
            };
        if !driver_ok {
            score -= 40;
            missing.push("PawnIO");
        }
        if !self.engine_found {
            score -= 30;
            missing.push("Prime95");
        }
        if !crate::setup::ycruncher_present() {
            score -= 10;
            missing.push("y-cruncher");
        }
        (score, missing)
    }

    /// Without rights the window has nothing to show: a single, plain invitation to give it some.
    pub(super) fn no_access_card(&mut self, ui_: &mut egui::Ui) {
        let dark = self.dark;
        let p = ui::palette(dark);
        let lang = self.lang;
        let t = move |k: Key| text::tr(lang, k);
        let mut start_helper = false;
        let waiting = self.waiting_for_helper.is_some();
        let err = self.machine_error.clone();
        let height = ui_.available_height();
        let surface = ui::Surface {
            margin: 24.0,
            ..ui::Surface::raised(dark)
        };
        ui::surface(ui_, dark, surface, |ui_| {
            ui_.set_min_height(height - 48.0);
            ui_.vertical_centered(|ui_| {
                ui_.add_space(((height - 48.0 - 330.0) / 2.0).max(8.0));
                w::lock(ui_, dark, 60.0);
                ui_.add_space(14.0);
                ui_.label(
                    egui::RichText::new(t(Key::NotElevatedTitle))
                        .size(22.0)
                        .strong(),
                );
                ui_.add_space(4.0);
                ui_.allocate_ui_with_layout(
                    egui::Vec2::new(480.0_f32.min(ui_.available_width()), 10.0),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui_| {
                        if let Some(e) = &err {
                            ui_.colored_label(p.danger, e);
                        } else {
                            ui_.add(
                                egui::Label::new(egui::RichText::new(t(Key::NotElevatedBody)))
                                    .wrap(),
                            );
                            ui_.add_space(4.0);
                            ui_.add(
                                egui::Label::new(
                                    egui::RichText::new(t(Key::HelperHint))
                                        .size(ui::TEXT_SMALL)
                                        .color(p.muted),
                                )
                                .wrap(),
                            );
                        }
                    },
                );
                if err.is_none() {
                    ui_.add_space(14.0);
                    ui_.allocate_ui_with_layout(
                        egui::Vec2::new(300.0, 52.0),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui_| {
                            if w::big_button(
                                ui_,
                                dark,
                                t(Key::StartHelper),
                                w::ButtonStyle::Accent,
                                !waiting,
                            )
                            .clicked()
                            {
                                start_helper = true;
                            }
                        },
                    );
                    if w::button(
                        ui_,
                        dark,
                        t(Key::RestartElevated),
                        w::ButtonStyle::Quiet,
                        true,
                    )
                    .clicked()
                        && os::relaunch_elevated()
                    {
                        std::process::exit(0);
                    }
                    if waiting {
                        ui_.add_space(6.0);
                        ui_.label(egui::RichText::new(t(Key::HelperWaiting)).color(p.accent));
                    }
                }
            });
        });
        if start_helper && os::relaunch_elevated_with(&format!("helper --pipe {WINDOW_PIPE}")) {
            self.waiting_for_helper = Some(Instant::now());
            self.started_helper = true;
        }
    }

    /// Writes every changed value through the checked path and says what the processor holds afterwards.
    pub(super) fn apply_pending(&mut self) {
        if self.edit.is_empty() {
            let msg = text::tr(self.lang, Key::NothingToApply);
            self.say_toast(msg);
            return;
        }
        let edits: Vec<(usize, i32)> = self.edit.iter().map(|(c, v)| (*c, *v)).collect();
        let mut last = String::new();
        let mut all_stuck = true;
        for (core, value) in edits {
            match self.write_core(core, value) {
                Ok(held) => {
                    all_stuck &= held == value;
                    last = text::core_set_result(self.lang, core, value, held);
                }
                Err(e) => {
                    all_stuck = false;
                    last = e;
                }
            }
        }
        self.edit.clear();
        self.refresh();
        let msg = if all_stuck {
            text::tr(self.lang, Key::PendingApplied).to_string()
        } else {
            last
        };
        self.say_toast(&msg);
    }

    /// One value, written through the helper or directly. A family nobody has checked proves the write path first.
    fn write_core(&mut self, core: usize, value: i32) -> Result<i32, String> {
        if self.is_preview() {
            return Err("a preview writes nothing".to_string());
        }
        if let Some(link) = &self.link {
            return link.set(core, value);
        }
        let Some(m) = &self.machine else {
            return Err(text::tr(self.lang, Key::NotElevatedTitle).to_string());
        };
        if pbo_core::families::needs_selftest(&m.family) {
            match m.selftest(core) {
                Ok((a, b)) if a == b => {}
                _ => return Err(text::tr(self.lang, Key::InfoSelftestFailed).to_string()),
            }
        }
        m.set_margin(core, value)
    }

    /// "On every Windows start": the values as they are now become the profile, and the logon task applies it.
    pub(super) fn set_every_start(&mut self, on: bool) {
        if self.is_preview() {
            return;
        }
        if on {
            let t = os::local_time();
            let at = format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
                t.year, t.month, t.day, t.hour, t.minute, t.second
            );
            if let Some(i) = &self.info {
                let _ = self.store.save_profile(&i.brand, &self.held, &at);
            }
        }
        let (tx, rx) = mpsc::channel();
        self.setup_busy = Some(rx);
        let arg = if on { "on" } else { "off" };
        std::thread::spawn(move || {
            let code = commands::autostart(&[arg.to_string()]);
            let _ = tx.send(code);
        });
    }

    /// The steps of the search Auto makes: the person's own, or those of the intensity.
    pub(super) fn auto_search(&self) -> Search {
        match self.auto_step {
            Some(n) => Search::stepping(n),
            None => match self.intensity.preselected_mode() {
                Mode::Coarse => Search::Coarse,
                Mode::Fine => Search::Fine,
                Mode::Steps(n) => Search::stepping(n as u32),
            },
        }
    }

    /// Where the search of a core that holds nothing begins: the person's choice, or what the processor family suggests.
    pub(super) fn effective_start(&self) -> i32 {
        if self.plan.start > 0 {
            -(self.plan.start as i32)
        } else {
            self.info.as_ref().map_or(-15, |i| i.suggested_start)
        }
    }

    pub(super) fn campaign_view(&self) -> Option<CampaignView> {
        let info = self.info.as_ref()?;
        let cores: Vec<u32> = if self.has_cores() {
            self.plan
                .cores_or_all(info.cores)
                .into_iter()
                .map(|c| c as u32)
                .collect()
        } else {
            // Nothing is chosen: nothing is to be done, and it takes no time.
            Vec::new()
        };
        let held: BTreeMap<u32, i32> = self
            .held
            .iter()
            .enumerate()
            .map(|(c, v)| (c as u32, *v))
            .collect();
        let driver_ready = self.info.is_some();
        let step = campaign::recommend(
            &cores,
            &held,
            &self.knowledge,
            info.deepest,
            driver_ready,
            self.engine_found,
            self.plan.guardband,
        );
        let phase = step.stage.phase_number();
        let left = |intensity: Intensity| {
            // The steps the person chose, or those of the intensity; and the safety the plan holds.
            let mode = self
                .auto_step
                .and_then(|n| Search::stepping(n).mode())
                .unwrap_or_else(|| intensity.preselected_mode());
            auto::estimate_with(
                intensity,
                mode,
                &cores,
                &held,
                &self.knowledge,
                info.deepest,
                self.plan.guardband,
                phase.max(1),
                self.effective_start(),
            )
        };
        let by_intensity = Intensity::ALL
            .into_iter()
            .map(|i| (i, left(i).total_minutes))
            .collect();
        let estimate = left(self.intensity);
        let fresh = self.knowledge.values().all(|k| k.best_passed.is_none());
        Some(CampaignView {
            step,
            phase,
            estimate,
            by_intensity,
            fresh,
        })
    }

    /// The next step of the campaign as a plan: the profile of that step, on the cores it names.
    pub(super) fn start_auto(&mut self) {
        let Some(view) = self.campaign_view() else {
            return;
        };
        if view.step.action == Action::OpenSetup {
            self.show_setup = true;
            return;
        }
        if !self.has_cores() {
            return;
        }
        let Some(profile) = profiles::by_id(view.step.profile.name()) else {
            return;
        };
        let mut plan = Plan::from_profile(&profile, &self.plan);
        // How long a test runs, and how far a step goes, follow the intensity and not the profile.
        plan.seconds_per_test = self.intensity.minutes(view.phase.max(1)) * 60;
        plan.rounds = 1;
        if !view.step.cores.is_empty() {
            plan.cores = view.step.cores.iter().map(|c| *c as usize).collect();
        }
        plan.search = if view.step.use_auto_tuner {
            self.auto_search()
        } else {
            Search::Off
        };
        // The run view calls the phase its step.
        self.next_phase = view.phase.max(1) as u32;
        self.start_with(plan);
    }

    pub(super) fn interrupted_strip(&mut self, ui_: &mut egui::Ui) {
        let dark = self.dark;
        let p = ui::palette(dark);
        let lang = self.lang;
        let t = move |k: Key| text::tr(lang, k);
        let busy = self.restore_busy.is_some();
        let mut restore = false;
        let surface = ui::Surface {
            margin: 12.0,
            ..ui::Surface::inset(dark).tinted(p.accent, 0.16)
        };
        ui::surface(ui_, dark, surface, |ui_| {
            ui_.label(
                egui::RichText::new(t(Key::InterruptedTitle))
                    .strong()
                    .color(p.accent),
            );
            ui_.label(
                egui::RichText::new(t(Key::InterruptedBody))
                    .size(ui::TEXT_SMALL)
                    .color(p.muted),
            );
            ui_.add_space(2.0);
            if w::button(
                ui_,
                dark,
                t(Key::InterruptedRestore),
                w::ButtonStyle::Accent,
                !busy,
            )
            .clicked()
            {
                restore = true;
            }
        });
        if restore && !self.is_preview() {
            let (tx, rx) = mpsc::channel();
            self.restore_busy = Some(rx);
            let link = self.link.clone();
            std::thread::spawn(move || {
                let code = match link {
                    Some(l) => i32::from(!l.resume()),
                    None => commands::resume(&[]),
                };
                let _ = tx.send(code);
            });
        }
    }

    pub(super) fn stop_run(&mut self) {
        match &self.link {
            Some(link) => link.stop(),
            None => os::request_cancel(),
        }
        self.command(Command::Stop);
    }

    pub(super) fn refresh_log_tail(&mut self) {
        let dir = self.store.logs_dir();
        let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("pbo_"))
            })
            .collect();
        files.sort();
        self.log_tail = match files.last().and_then(|f| std::fs::read(f).ok()) {
            Some(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                let lines: Vec<String> = text.lines().map(str::to_string).collect();
                let skip = lines.len().saturating_sub(400);
                lines.into_iter().skip(skip).collect()
            }
            None => Vec::new(),
        };
    }
    // ── Windows: the BIOS list and Driver & Engines ───────────────────────

    pub(super) fn windows(&mut self, ctx: &egui::Context) {
        let dark = self.dark;
        let lang = self.lang;
        let t = move |k: Key| text::tr(lang, k);

        if self.show_bios {
            let mut open = true;
            let mut copy = false;
            let values = self.held.clone();
            egui::Window::new(t(Key::BiosTitle))
                .open(&mut open)
                .collapsible(false)
                .vscroll(true)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui_| {
                    ui_.set_width(420.0);
                    ui_.label(t(Key::BiosBody));
                    ui_.add_space(8.0);
                    for (c, v) in values.iter().enumerate() {
                        let line = if *v < 0 {
                            format!("Core {c:>2}   {v:>4}   ({}, {})", t(Key::BiosNegative), -v)
                        } else {
                            format!("Core {c:>2}   {v:>4}")
                        };
                        ui_.label(egui::RichText::new(line).monospace());
                    }
                    ui_.add_space(8.0);
                    if w::button(ui_, self.dark, t(Key::Copy), w::ButtonStyle::Accent, true)
                        .clicked()
                    {
                        copy = true;
                    }
                });
            if copy {
                let text = values
                    .iter()
                    .enumerate()
                    .map(|(c, v)| format!("Core {c}: {v}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                ctx.copy_text(text);
                self.say_toast(t(Key::Copied));
            }
            self.show_bios = open;
        }

        if self.show_setup {
            let mut open = true;
            egui::Window::new(t(Key::DriverEngines))
                .open(&mut open)
                .collapsible(false)
                .vscroll(true)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui_| {
                    ui_.set_width(460.0);
                    self.setup_content(ui_, dark);
                });
            self.show_setup = open;
        }
    }

    /// "Ready for a long run?": the checks of the machine, and forgetting what the program has learned.
    fn readiness_content(&mut self, ui_: &mut egui::Ui, dark: bool) {
        let lang = self.lang;
        let t = move |k: Key| text::tr(lang, k);
        let p = ui::palette(dark);
        let mut check = false;
        let mut forget = false;
        ui::row(ui_, t(Key::ReadyTitle), None, |ui_| {
            if w::button(ui_, dark, t(Key::ReadyCheck), w::ButtonStyle::Quiet, true).clicked() {
                check = true;
            }
        });
        ui::row(ui_, t(Key::LearnedTitle), None, |ui_| {
            if w::button(ui_, dark, t(Key::ForgetMemory), w::ButtonStyle::Quiet, true).clicked() {
                forget = true;
            }
        });
        if let Some(list) = self.readiness.clone() {
            ui_.add_space(4.0);
            for c in &list {
                let (label, hint) = match c.item {
                    pbo_core::readiness::Item::LogonTask => {
                        (Key::ReadyLogonTask, Key::ReadyLogonTaskHint)
                    }
                    pbo_core::readiness::Item::AutoLogin => {
                        (Key::ReadyAutoLogin, Key::ReadyAutoLoginHint)
                    }
                    pbo_core::readiness::Item::Sleep => (Key::ReadySleep, Key::ReadySleepHint),
                    pbo_core::readiness::Item::Disk => (Key::ReadyDisk, Key::ReadyDiskHint),
                    pbo_core::readiness::Item::Temperature => {
                        (Key::ReadyTemperature, Key::ReadyTemperatureHint)
                    }
                };
                ui_.horizontal(|ui_| {
                    let (kind, mark) = match c.level {
                        pbo_core::readiness::Level::Ok => (ChipKind::Ok, "ok"),
                        pbo_core::readiness::Level::Warn => (ChipKind::Accent, "!"),
                        pbo_core::readiness::Level::Unknown => (ChipKind::Muted, "?"),
                    };
                    ui::chip(ui_, dark, mark, kind);
                    ui_.label(t(label));
                });
                if c.level == pbo_core::readiness::Level::Warn {
                    ui_.add(
                        egui::Label::new(
                            egui::RichText::new(t(hint))
                                .size(ui::TEXT_SMALL)
                                .color(p.muted),
                        )
                        .wrap(),
                    );
                }
            }
        }
        if check {
            let held: BTreeMap<u32, i32> = self
                .held
                .iter()
                .enumerate()
                .map(|(c, v)| (c as u32, *v))
                .collect();
            let (cores, chip) = self
                .info
                .as_ref()
                .map_or((0, -30), |i| (i.cores, i.deepest));
            let hours =
                self.plan
                    .worst_case_seconds(cores, &held, &self.knowledge, chip) as f64
                    / 3600.0;
            self.readiness = Some(pbo_core::readiness::evaluate(&crate::ready::gather(hours)));
        }
        if forget && !self.is_preview() {
            let _ = std::fs::remove_file(self.store.root.join("core_knowledge.json"));
            self.knowledge.clear();
            self.say_toast(t(Key::Forgotten));
        }
    }

    fn setup_content(&mut self, ui_: &mut egui::Ui, dark: bool) {
        let lang = self.lang;
        let t = move |k: Key| text::tr(lang, k);
        let mut install: Option<&str> = None;
        let busy = self.setup_busy.is_some();
        // A window without rights has no way to open the driver itself, but one that has a helper (which has the rights and read
        // this very processor through it) has proof that the driver works.
        let driver = if self.link.is_some() && self.info.is_some() {
            Ok(())
        } else {
            pbo_smu::driver_status()
        };
        let rows = [
            ("Prime95", "prime95", crate::setup::prime95_present()),
            ("y-cruncher", "ycruncher", crate::setup::ycruncher_present()),
        ];
        let (brand, family, bios) = match &self.info {
            Some(i) => (
                i.brand.clone(),
                i.family_name.clone(),
                os::bios_version().unwrap_or_default(),
            ),
            None => (String::new(), String::new(), String::new()),
        };
        ui::row(ui_, t(Key::InfoProcessor), None, |ui_| {
            ui_.label(format!("{brand} ({family})"));
        });
        ui::row(ui_, t(Key::InfoBios), None, |ui_| {
            ui_.label(&bios);
        });
        ui_.separator();
        for (label, key, ok) in rows {
            ui::row(ui_, label, None, |ui_| {
                if ok {
                    ui::chip(ui_, dark, t(Key::SetupPresent), ChipKind::Ok);
                } else {
                    if w::button(
                        ui_,
                        dark,
                        t(Key::SetupInstall),
                        w::ButtonStyle::Accent,
                        !busy,
                    )
                    .clicked()
                    {
                        install = Some(key);
                    }
                    ui::chip(ui_, dark, t(Key::SetupMissing), ChipKind::Accent);
                }
            });
        }
        ui::row(ui_, t(Key::SetupDriver), None, |ui_| match &driver {
            Ok(()) => {
                ui::chip(ui_, dark, t(Key::SetupPresent), ChipKind::Ok);
            }
            Err(pbo_smu::Error::DriverUnavailable(5)) => {
                ui::chip(ui_, dark, t(Key::SetupDriverUnknown), ChipKind::Muted);
            }
            Err(_) => {
                if w::button(
                    ui_,
                    dark,
                    t(Key::SetupInstall),
                    w::ButtonStyle::Accent,
                    !busy,
                )
                .clicked()
                {
                    install = Some("pawnio");
                }
                ui::chip(ui_, dark, t(Key::SetupMissing), ChipKind::Accent);
            }
        });
        ui_.separator();
        self.readiness_content(ui_, dark);
        ui_.separator();
        let mut run_selftest = false;
        ui::row(ui_, t(Key::InfoWritePath), None, |ui_| {
            if w::button(
                ui_,
                dark,
                t(Key::InfoRunSelftest),
                w::ButtonStyle::Quiet,
                true,
            )
            .clicked()
            {
                run_selftest = true;
            }
        });
        if run_selftest && !self.is_preview() {
            if let Some(machine) = &self.machine {
                let result = (0..machine.cores).try_for_each(|c| match machine.selftest(c) {
                    Ok((a, b)) if a == b => Ok(()),
                    Ok((a, b)) => Err(format!("core {c}: wrote {a}, read back {b}")),
                    Err(e) => Err(e),
                });
                self.selftest = Some(result);
            }
        }
        match &self.selftest {
            Some(Ok(())) => {
                ui::chip(ui_, dark, t(Key::InfoSelftestOk), ChipKind::Ok);
            }
            Some(Err(e)) => {
                ui::chip(ui_, dark, t(Key::InfoSelftestFailed), ChipKind::Danger);
                ui_.label(e);
            }
            None => {}
        }
        if let Some(which) = install.filter(|_| !self.is_preview()) {
            let (tx, rx) = mpsc::channel();
            self.setup_busy = Some(rx);
            let which = which.to_string();
            std::thread::spawn(move || {
                let code = crate::setup::run(&[which]);
                let _ = tx.send(code);
            });
        }
    }
}
