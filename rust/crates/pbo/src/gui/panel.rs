//! The right side: three tabs. Auto does the whole job and needs one decision (how thorough); Tests runs one ready-made
//! test list; Advanced builds a plan by hand. While a run is going or has just ended, the Auto tab shows that instead.
//!
//! Everything here fits the column without scrolling at the size the window opens with; a scroll area stays as the
//! fallback for the smallest window.

use super::*;
use pbo_core::campaign::Action;
use pbo_core::profiles;
use pbo_ui::widgets::{self as w, Step as Stepper, StepState};

/// A paragraph of the given size that wraps inside the column.
fn paragraph(ui_: &mut egui::Ui, text: &str, size: f32, color: egui::Color32) {
    ui_.add(egui::Label::new(egui::RichText::new(text).size(size).color(color)).wrap());
}

impl App {
    pub(super) fn right_panel(&mut self, ui_: &mut egui::Ui) {
        let dark = self.dark;
        let lang = self.lang;
        let t = move |k: Key| text::tr(lang, k);
        let height = ui_.available_height();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui_, |ui_| {
                ui_.set_max_width(ui_.available_width());
                ui::surface(ui_, dark, ui::Surface::raised(dark), |ui_| {
                    ui_.set_min_height(height - 30.0);
                    let labels = [t(Key::TabAuto), t(Key::TabTests), t(Key::TabAdvanced)];
                    let mut tab = self.tab.min(2);
                    w::segmented(ui_, dark, &mut tab, &labels);
                    self.tab = tab;
                    ui_.add_space(6.0);
                    match self.tab {
                        1 => self.tests_tab(ui_),
                        2 => self.advanced_tab(ui_),
                        _ => self.auto_tab(ui_),
                    }
                });
            });
    }

    // ── Auto ──────────────────────────────────────────────────────────────

    fn auto_tab(&mut self, ui_: &mut egui::Ui) {
        let controls = Controls::of(&self.state);
        if controls.shows_progress {
            self.run_view(ui_);
            return;
        }
        if controls.shows_result {
            self.result_view(ui_);
            return;
        }
        let dark = self.dark;
        let p = ui::palette(dark);
        let lang = self.lang;
        let t = move |k: Key| text::tr(lang, k);
        let Some(view) = self.campaign_view() else {
            return;
        };
        let setup_needed = view.step.action == Action::OpenSetup;
        // Where the room is short, the explanations make way for what can be done.
        let room = ui_.available_height();
        // With the fine-tuning open there is more to show: the explanations make way earlier, and the tiles and the steps
        // shrink, so that nothing ever needs a scroll bar.
        let tight = self.auto_more && room < 560.0;
        let compact = self.interrupted || self.pending_continue.is_some() || room < 470.0 || tight;

        let title = if setup_needed {
            t(Key::CampaignSetup)
        } else if view.fresh && view.phase <= 1 {
            t(Key::CampaignReady)
        } else {
            t(Key::CampaignInProgress)
        };
        ui_.horizontal(|ui_| {
            ui_.label(
                egui::RichText::new(title)
                    .size(if tight { 16.0 } else { 19.0 })
                    .strong()
                    .color(p.ok),
            );
            // The fine-tuning is opened at the bottom and closed here, where it does not take a line of its own.
            if self.auto_more {
                ui_.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui_| {
                    if w::button_sized(
                        ui_,
                        dark,
                        t(Key::AutoLess),
                        w::ButtonStyle::Quiet,
                        true,
                        26.0,
                    )
                    .clicked()
                    {
                        self.auto_more = false;
                    }
                });
            }
        });
        let body = if setup_needed {
            t(Key::CampaignSetupBody)
        } else if self.interrupted {
            t(Key::CampaignResume)
        } else if view.fresh {
            t(Key::CampaignFresh)
        } else {
            t(Key::StopAnyTime)
        };
        // After a crash that Auto is carried on after: how long it takes, and the way to stop it.
        if self.pending_continue.is_some() {
            ui_.add_space(4.0);
            self.continue_strip(ui_);
            ui_.add_space(8.0);
            // Nothing else to choose meanwhile: Auto goes on the way it was started. One button goes on now.
            if w::big_button(ui_, dark, t(Key::Continue), w::ButtonStyle::Accent, true).clicked() {
                self.start_or_go_on_now();
            }
            return;
        }
        // After a crash the strip that offers to put the values back comes first.
        if self.interrupted {
            ui_.add_space(4.0);
            self.interrupted_strip(ui_);
            ui_.add_space(6.0);
        } else if !compact && (setup_needed || view.fresh) {
            paragraph(ui_, body, ui::TEXT_BODY, p.muted);
            ui_.add_space(4.0);
        }

        // How Auto goes about it, chosen beforehand: how thorough, how far each step goes, how much safety is kept and where
        // the search begins. What is selected is what the program has judged efficient for this processor; each is one click
        // away, and the times follow.
        if !setup_needed {
            if !compact {
                w::section_label(ui_, dark, t(Key::IntensityTitle));
            }
            let gap = 8.0;
            let tile_w = (ui_.available_width() - 2.0 * gap) / 3.0;
            let mut pick = None;
            ui_.horizontal(|ui_| {
                ui_.spacing_mut().item_spacing.x = gap;
                for (intensity, minutes) in &view.by_intensity {
                    let (title, hint) = match intensity {
                        Intensity::Fast => (t(Key::IntensityFast), t(Key::IntensityFastHint)),
                        Intensity::Normal => (t(Key::IntensityNormal), t(Key::IntensityNormalHint)),
                        Intensity::Thorough => {
                            (t(Key::IntensityThorough), t(Key::IntensityThoroughHint))
                        }
                    };
                    let on = *intensity == self.intensity;
                    let r = w::tile(
                        ui_,
                        dark,
                        on,
                        egui::Vec2::new(tile_w, if tight { 44.0 } else { 80.0 }),
                        |ui_| {
                            ui_.label(egui::RichText::new(title).strong().color(if on {
                                p.accent
                            } else {
                                p.text
                            }));
                            ui_.label(
                                egui::RichText::new(text::span_short(*minutes))
                                    .monospace()
                                    .size(13.0)
                                    .color(if on { p.text } else { p.muted }),
                            );
                            if !tight {
                                paragraph(ui_, hint, ui::TEXT_SMALL - 1.0, p.muted);
                            }
                        },
                    );
                    if r.clicked() {
                        pick = Some(*intensity);
                    }
                }
            });
            if let Some(i) = pick {
                self.intensity = i;
            }

            // Step, safety and start are chosen beforehand and out of sight: Auto starts as it stands. Whoever wants to play
            // with them opens them, and the button that opened them goes away.
            if self.auto_more {
                let spacing = ui_.spacing().item_spacing.y;
                ui_.spacing_mut().item_spacing.y = 4.0;
                let default_step =
                    pbo_core::tuner::step_size(self.intensity.preselected_mode()) as u32;
                let step_now = self.auto_search().step_size().unwrap_or(default_step);
                let family_start = self.info.as_ref().map_or(15, |i| -i.suggested_start) as u32;
                let start_now = (-self.effective_start()) as u32;
                // The start only matters for a core that holds nothing yet; with every core on a value it changes no time.
                let start_matters = self.info.as_ref().is_some_and(|i| {
                    self.plan.cores_or_all(i.cores).iter().any(|c| {
                        self.held.get(*c).is_some_and(|h| *h >= 0)
                            && self
                                .knowledge
                                .get(&(*c as u32))
                                .is_none_or(|k| k.best_passed.is_none())
                    })
                });
                let steps = ["1", "2", "3", "4", "5"];
                let starts = ["−10", "−15", "−20"];
                if let Some(i) = Self::choice_row(
                    ui_,
                    dark,
                    t(Key::AutoStep),
                    t(Key::AutoStepTip),
                    &steps,
                    (1..=5).position(|n| n == step_now),
                ) {
                    let n = i as u32 + 1;
                    self.auto_step = (n != default_step).then_some(n);
                }
                if let Some(i) = Self::choice_row(
                    ui_,
                    dark,
                    t(Key::AutoMargin),
                    t(Key::AutoMarginTip),
                    &steps,
                    (1..=5).position(|n| n == self.plan.guardband),
                ) {
                    self.plan.guardband = i as i32 + 1;
                }
                let start_tip = if start_matters {
                    t(Key::AutoStartTip).to_string()
                } else {
                    format!("{} {}", t(Key::AutoStartTip), t(Key::AutoStartNoEffect))
                };
                if let Some(i) = Self::choice_row(
                    ui_,
                    dark,
                    t(Key::AutoStart),
                    &start_tip,
                    &starts,
                    [10u32, 15, 20].iter().position(|n| *n == start_now),
                ) {
                    let n = [10u32, 15, 20][i];
                    self.plan.start = if n == family_start { 0 } else { n };
                }
                let after_crash = usize::from(self.logon_continue);
                if let Some(i) = Self::choice_row(
                    ui_,
                    dark,
                    t(Key::AutoAfterCrash),
                    t(Key::AutoAfterCrashTip),
                    &[t(Key::AutoWait), t(Key::AutoCarryOn)],
                    Some(after_crash),
                ) {
                    self.set_continue_after_crash(i == 1);
                }
                ui_.spacing_mut().item_spacing.y = spacing;
            }
            if !compact {
                paragraph(ui_, t(Key::IntensityNote), ui::TEXT_SMALL - 1.0, p.muted);
            }
            if self.two_way_now() {
                paragraph(ui_, t(Key::AutoIsTwoWay), ui::TEXT_SMALL - 1.0, p.accent);
            }
            ui_.add_space(4.0);
        }

        // The three steps, each with what it costs at the chosen thoroughness.
        let items: Vec<(StepState, &str, String)> =
            [(1, Key::Step1), (2, Key::Step2), (3, Key::Step3)]
                .into_iter()
                .map(|(n, key)| {
                    let state = if setup_needed || n > view.phase {
                        StepState::Pending
                    } else if n < view.phase {
                        StepState::Done
                    } else {
                        StepState::Current
                    };
                    let detail = match state {
                        StepState::Done => t(Key::PhaseDone).to_string(),
                        _ if setup_needed => String::new(),
                        _ => view
                            .estimate
                            .per_phase
                            .iter()
                            .find(|(phase, _)| *phase == n)
                            .map_or(String::new(), |(_, m)| text::up_to(lang, *m)),
                    };
                    (state, t(key), detail)
                })
                .collect();
        w::timeline(
            ui_,
            dark,
            &items,
            if tight {
                31.0
            } else if compact {
                38.0
            } else {
                44.0
            },
        );
        ui_.add_space(2.0);

        let can = Controls::of(&self.state).can_start && (setup_needed || self.has_cores());
        let label = if setup_needed {
            t(Key::DriverEngines)
        } else if view.fresh && view.phase <= 1 {
            t(Key::Start)
        } else {
            t(Key::Continue)
        };
        if w::big_button(ui_, dark, label, w::ButtonStyle::Accent, can).clicked() {
            self.start_or_go_on_now();
        }
        if !self.has_cores() && !setup_needed {
            paragraph(ui_, t(Key::NoCoreChosen), ui::TEXT_SMALL, p.danger);
        } else if !compact && ui_.available_height() > 30.0 {
            let hint = if self.logon_continue {
                t(Key::RestartHintCarryOn)
            } else {
                t(Key::RestartHint)
            };
            paragraph(ui_, hint, ui::TEXT_SMALL - 1.0, p.muted);
        }
        if !self.auto_more && !setup_needed && self.has_cores() {
            ui_.add_space(2.0);
            if w::button(ui_, dark, t(Key::AutoMore), w::ButtonStyle::Quiet, true).clicked() {
                self.auto_more = true;
            }
        }
    }

    /// Whether a core holds a value it is known to fail at: the case in which Auto goes back up.
    fn two_way_now(&self) -> bool {
        self.held.iter().enumerate().any(|(core, held)| {
            self.knowledge
                .get(&(core as u32))
                .is_some_and(|k| k.is_known_bad(*held))
        })
    }

    /// The run in progress: what is being tested, how far, and the one button that stops it.
    fn run_view(&mut self, ui_: &mut egui::Ui) {
        let dark = self.dark;
        let p = ui::palette(dark);
        let lang = self.lang;
        let t = move |k: Key| text::tr(lang, k);
        let state_text = match &self.state {
            State::Preparing => t(Key::StatePreparing).to_string(),
            State::Stopping => t(Key::StateStopping).to_string(),
            State::Running { phase, core } => {
                let line = text::progress(lang, *phase, *core);
                match (self.run_seen.len(), self.run_cores) {
                    (n, total) if total > 1 && n >= 1 => {
                        format!(
                            "{line} \u{b7} {}",
                            text::core_of_cores(lang, n.min(total), total)
                        )
                    }
                    _ => line,
                }
            }
            _ => String::new(),
        };
        ui_.label(
            egui::RichText::new(t(Key::RunningTitle))
                .size(19.0)
                .strong()
                .color(p.ok),
        );
        ui_.add_space(2.0);
        ui_.label(egui::RichText::new(state_text).size(16.0).strong());
        if let Some((_, margin)) = self.current {
            ui_.label(
                egui::RichText::new(format!("{}: {margin}", t(Key::ValueUnderTest))).color(p.muted),
            );
        }
        ui_.add_space(8.0);
        if self.test_seconds > 0 {
            let elapsed = self.test_began.elapsed().as_secs_f32();
            let done = (elapsed / self.test_seconds as f32).min(1.0);
            w::progress_bar(ui_, dark, done, None);
            // "0:22 / 1:00": the hours are left out while there are none.
            let clock = |seconds: i64| {
                pbo_core::duration::hours_seconds(seconds)
                    .trim_start_matches("0:")
                    .to_string()
            };
            ui_.label(
                egui::RichText::new(format!(
                    "{} / {}",
                    clock(elapsed as i64),
                    clock(self.test_seconds as i64)
                ))
                .monospace()
                .size(ui::TEXT_SMALL)
                .color(p.muted),
            );
            ui_.ctx().request_repaint_after(Duration::from_millis(250));
        } else {
            // A time of "auto": the end is when the work list has been through once, so there is no share to show.
            w::progress_bar(ui_, dark, 0.0, None);
            if self.current.is_some() {
                let elapsed = self.test_began.elapsed().as_secs() as i64;
                let clock = pbo_core::duration::hours_seconds(elapsed);
                ui_.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        clock.trim_start_matches("0:"),
                        t(Key::SecondsAuto).to_lowercase()
                    ))
                    .monospace()
                    .size(ui::TEXT_SMALL)
                    .color(p.muted),
                );
                ui_.ctx().request_repaint_after(Duration::from_millis(500));
            }
        }
        ui_.add_space(10.0);
        let can_stop = Controls::of(&self.state).can_stop;
        if w::big_button(ui_, dark, t(Key::Stop), w::ButtonStyle::Danger, can_stop).clicked() {
            self.stop_run();
        }
        ui_.add_space(8.0);
        // The last lines of the log, so something is seen to happen.
        let lines: Vec<String> = self.log.iter().rev().take(7).rev().cloned().collect();
        let surface = ui::Surface {
            margin: 10.0,
            ..ui::Surface::inset(dark)
        };
        ui::surface(ui_, dark, surface, |ui_| {
            for l in lines {
                ui_.add(
                    egui::Label::new(
                        egui::RichText::new(l)
                            .monospace()
                            .size(ui::TEXT_SMALL - 1.0)
                            .color(p.muted),
                    )
                    .truncate(),
                );
            }
        });
    }

    fn result_view(&mut self, ui_: &mut egui::Ui) {
        let dark = self.dark;
        let p = ui::palette(dark);
        let lang = self.lang;
        let t = move |k: Key| text::tr(lang, k);
        let headline = match &self.state {
            State::Finished => t(Key::StateFinished),
            State::Halted(HaltReason::StoppedByPerson) => t(Key::StateStopped),
            State::Halted(HaltReason::LimitReached) => t(Key::StateLimit),
            State::Halted(HaltReason::Inconclusive) => t(Key::StateInconclusive),
            _ => "",
        };
        ui_.label(
            egui::RichText::new(headline)
                .size(19.0)
                .strong()
                .color(p.ok),
        );
        ui_.add_space(6.0);
        let surface = ui::Surface {
            margin: 12.0,
            ..ui::Surface::inset(dark)
        };
        ui::surface(ui_, dark, surface, |ui_| {
            ui_.horizontal_wrapped(|ui_| {
                for (core, v) in self.held.iter().enumerate() {
                    ui::chip(ui_, dark, &format!("{core}: {v}"), ChipKind::Info);
                }
            });
        });
        ui_.add_space(10.0);
        let mut keep = false;
        let mut done = false;
        if w::big_button(ui_, dark, t(Key::Dismiss), w::ButtonStyle::Accent, true).clicked() {
            done = true;
        }
        // The values are gone after a restart unless the UEFI has them: the way to enter them is one click away.
        if w::button(ui_, dark, t(Key::EnterInBios), w::ButtonStyle::Quiet, true).clicked() {
            self.show_bios = true;
        }
        if w::button(
            ui_,
            dark,
            t(Key::SaveAsLogonProfile),
            w::ButtonStyle::Quiet,
            true,
        )
        .clicked()
        {
            keep = true;
        }
        if keep && !self.is_preview() {
            if let Some(m) = &self.info {
                let tm = os::local_time();
                let at = format!(
                    "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
                    tm.year, tm.month, tm.day, tm.hour, tm.minute, tm.second
                );
                let _ = self.store.save_profile(&m.brand, &self.held, &at);
                let msg = t(Key::Saved);
                self.say_toast(msg);
            }
        }
        paragraph(ui_, t(Key::EnterInBiosHint), ui::TEXT_SMALL - 1.0, p.muted);
        if done {
            self.command(Command::Dismiss);
        }
    }

    // ── Tests ─────────────────────────────────────────────────────────────

    fn tests_tab(&mut self, ui_: &mut egui::Ui) {
        let dark = self.dark;
        let p = ui::palette(dark);
        let lang = self.lang;
        let t = move |k: Key| text::tr(lang, k);
        paragraph(ui_, t(Key::TestsIntro), ui::TEXT_SMALL, p.muted);
        ui_.add_space(2.0);
        let list = profiles::all();
        let mut chosen = self.profile_pick.min(list.len().saturating_sub(1));
        let gap = 8.0;
        let col_w = (ui_.available_width() - gap) / 2.0;
        for (row, pair) in list.chunks(2).enumerate() {
            ui_.horizontal(|ui_| {
                ui_.spacing_mut().item_spacing = egui::Vec2::new(gap, 6.0);
                for (j, prof) in pair.iter().enumerate() {
                    let i = row * 2 + j;
                    if w::pill(
                        ui_,
                        dark,
                        text::profile_short(lang, prof.id),
                        i == chosen,
                        col_w,
                    )
                    .clicked()
                    {
                        chosen = i;
                    }
                }
            });
        }
        self.profile_pick = chosen;
        ui_.add_space(4.0);

        // What the chosen list is, in words and numbers.
        let Some(prof) = list.get(chosen) else {
            return;
        };
        let (title, hint) = text::profile_text(lang, prof.id);
        let cores = self.info.as_ref().map_or(0, |i| {
            if self.has_cores() {
                self.plan.cores_or_all(i.cores).len()
            } else {
                0
            }
        }) as u32;
        let minutes = cores * prof.minutes_per_core * prof.iterations * prof.legs.len() as u32;
        let surface = ui::Surface {
            margin: 12.0,
            ..ui::Surface::inset(dark)
        };
        ui::surface(ui_, dark, surface, |ui_| {
            ui_.horizontal(|ui_| {
                ui_.add(egui::Label::new(egui::RichText::new(title).strong()).wrap());
            });
            if prof.essential {
                ui::chip(ui_, dark, t(Key::Recommended), ChipKind::Accent);
            }
            paragraph(ui_, hint, ui::TEXT_SMALL, p.muted);
            ui_.add(
                egui::Label::new(
                    egui::RichText::new(text::profile_meta(lang, prof))
                        .monospace()
                        .size(ui::TEXT_SMALL - 1.0)
                        .color(p.muted),
                )
                .wrap(),
            );
            if cores > 0 {
                ui_.label(
                    egui::RichText::new(format!(
                        "{} {} {}",
                        t(Key::TestsTakes),
                        text::span(lang, f64::from(minutes)),
                        t(Key::TestsOnCores)
                    ))
                    .size(ui::TEXT_SMALL)
                    .strong()
                    .color(p.accent),
                );
            }
        });
        ui_.add_space(6.0);
        let can = Controls::of(&self.state).can_start && self.engine_found && self.has_cores();
        if w::big_button(ui_, dark, t(Key::RunThisTest), w::ButtonStyle::Accent, can).clicked() {
            if let Some(prof) = list.get(chosen) {
                let plan = Plan::from_profile(prof, &self.plan);
                self.start_with(plan);
            }
        }
        if !self.has_cores() {
            paragraph(ui_, t(Key::NoCoreChosen), ui::TEXT_SMALL, p.danger);
        }
    }

    // ── Advanced ──────────────────────────────────────────────────────────

    /// What to test: the program, what it computes, for how long, with how many threads.
    fn adv_test_page(&mut self, ui_: &mut egui::Ui) {
        use pbo_core::engine::{FftPreset, Prime95Mode, YcSet};
        let dark = self.dark;
        let lang = self.lang;
        let t = move |k: Key| text::tr(lang, k);
        Self::labelled(ui_, dark, t(Key::EngineLabel), |ui_| {
            let mut idx = usize::from(self.plan.engine == Engine::YCruncher);
            if w::segmented(ui_, dark, &mut idx, &["Prime95", "y-cruncher"]) {
                self.plan.engine = if idx == 1 {
                    Engine::YCruncher
                } else {
                    Engine::Prime95
                };
            }
        });
        if self.plan.engine == Engine::YCruncher {
            Self::labelled(ui_, dark, t(Key::AdvYcSet), |ui_| {
                let mut idx = usize::from(self.plan.yc_set == YcSet::FastDiscovery);
                if w::segmented(
                    ui_,
                    dark,
                    &mut idx,
                    &[t(Key::YcSetCurve), t(Key::YcSetFast)],
                ) {
                    self.plan.yc_set = YcSet::ALL[idx];
                }
            });
        } else {
            Self::labelled(ui_, dark, t(Key::AdvMode), |ui_| {
                let modes = [
                    Prime95Mode::Sse,
                    Prime95Mode::Avx,
                    Prime95Mode::Avx2,
                    Prime95Mode::Avx512,
                ];
                let mut idx = modes.iter().position(|m| *m == self.plan.mode).unwrap_or(2);
                if w::segmented(ui_, dark, &mut idx, &["SSE", "AVX", "AVX2", "AVX-512"]) {
                    self.plan.mode = modes[idx];
                }
            });
            // The caption of the range; with a range of one's own, its two numbers sit at the right end of the same line.
            let mut own = None;
            ui_.horizontal(|ui_| {
                w::section_label(ui_, dark, t(Key::FftLabel));
                if let FftPreset::Custom(lo0, hi0) = self.plan.fft {
                    let (mut lo, mut hi) = (lo0, hi0);
                    ui_.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui_| {
                        ui_.spacing_mut().item_spacing.x = 6.0;
                        ui_.add(
                            egui::DragValue::new(&mut hi)
                                .range(lo..=pbo_core::engine::FFT_MAX_K)
                                .suffix(" K")
                                .speed(8.0),
                        );
                        ui_.label(t(Key::FftTo));
                        ui_.add(
                            egui::DragValue::new(&mut lo)
                                .range(pbo_core::engine::FFT_MIN_K..=hi)
                                .suffix(" K")
                                .speed(8.0),
                        );
                        ui_.label(t(Key::FftFrom));
                    });
                    if (lo, hi) != (lo0, hi0) {
                        own = FftPreset::custom(lo, hi);
                    }
                }
            });
            if let Some(f) = own {
                self.plan.fft = f;
            }
            let presets = [
                FftPreset::Smallest,
                FftPreset::Small,
                FftPreset::Large,
                FftPreset::Huge,
                FftPreset::All,
                FftPreset::Heavy,
                FftPreset::HeavyShort,
                FftPreset::Moderate,
            ];
            let gap = 6.0;
            let pill_w = (ui_.available_width() - 2.0 * gap) / 3.0;
            let custom = matches!(self.plan.fft, FftPreset::Custom(..));
            let mut items: Vec<(String, bool)> = presets
                .iter()
                .map(|f| (f.name().to_string(), self.plan.fft == *f))
                .collect();
            items.push((t(Key::FftCustom).to_string(), custom));
            for (row, chunk) in items.chunks(3).enumerate() {
                ui_.horizontal(|ui_| {
                    ui_.spacing_mut().item_spacing.x = gap;
                    for (col, (label, on)) in chunk.iter().enumerate() {
                        if w::pill(ui_, dark, label, *on, pill_w).clicked() {
                            let i = row * 3 + col;
                            self.plan.fft = match presets.get(i) {
                                Some(f) => *f,
                                None => {
                                    let (lo, hi) = self.plan.fft.range();
                                    FftPreset::custom(lo, hi).unwrap_or(FftPreset::Custom(4, 1344))
                                }
                            };
                        }
                    }
                });
            }
        }

        // For how long: the usual lengths as pills, and "auto". Any other length from a plan file stays and is shown in
        // the command line.
        w::section_label(ui_, dark, t(Key::SecondsLabel));
        // For how long: the usual lengths as pills, and "auto" (until the test program has been through its work list
        // once, an hour at most). Any other length from a plan file stays and is shown in the command line.
        let lengths: [(u32, &str); 7] = [
            (30, "30 s"),
            (60, "1 min"),
            (120, "2 min"),
            (300, "5 min"),
            (600, "10 min"),
            (1800, "30 min"),
            (0, t(Key::SecondsAuto)),
        ];
        let gap = 6.0;
        let pill_w = (ui_.available_width() - 3.0 * gap) / 4.0;
        for chunk in lengths.chunks(4) {
            ui_.horizontal(|ui_| {
                ui_.spacing_mut().item_spacing.x = gap;
                for (seconds, label) in chunk {
                    let r = w::pill(
                        ui_,
                        dark,
                        label,
                        self.plan.seconds_per_test == *seconds,
                        pill_w,
                    );
                    let r = if *seconds == 0 {
                        r.on_hover_text(t(Key::SecondsAutoTip))
                    } else {
                        r
                    };
                    if r.clicked() {
                        self.plan.seconds_per_test = *seconds;
                    }
                }
            });
        }

        // How often it goes round, and with how many threads.
        ui_.horizontal(|ui_| {
            ui_.spacing_mut().item_spacing.x = 8.0;
            ui_.add_sized(
                [100.0, 30.0],
                egui::Label::new(
                    egui::RichText::new(t(Key::AdvRounds)).color(ui::palette(dark).muted),
                )
                .truncate(),
            );
            self.plan.rounds = Self::count_stepper(ui_, dark, self.plan.rounds, 1, 50);
            ui_.add_space(10.0);
            w::checkbox(ui_, dark, &mut self.plan.smt, t(Key::AdvThreads));
        });
    }

    /// How to run it: how the load is interrupted, the order of the cores, the search, what happens at an error.
    fn adv_run_page(&mut self, ui_: &mut egui::Ui) {
        use pbo_core::onerror::OnError;
        let dark = self.dark;
        let p = ui::palette(dark);
        let lang = self.lang;
        let t = move |k: Key| text::tr(lang, k);
        Self::labelled_tip(
            ui_,
            dark,
            t(Key::AdvTransient),
            t(Key::TransientTip),
            |ui_| {
                use pbo_core::transient::Transient;
                let labels = [
                    t(Key::TransientOff),
                    t(Key::TransientPauses),
                    t(Key::TransientBursts),
                    t(Key::TransientGame),
                ];
                let mut idx = match self.plan.transient {
                    Transient::Off => 0,
                    Transient::Periodic { .. } => 1,
                    Transient::Bursts => 2,
                    Transient::Game => 3,
                };
                if w::segmented(ui_, dark, &mut idx, &labels) {
                    self.plan.transient = match idx {
                        0 => Transient::Off,
                        // Keeps the length of a pause a ready-made list brought along.
                        1 if matches!(self.plan.transient, Transient::Periodic { .. }) => {
                            self.plan.transient
                        }
                        1 => Transient::default(),
                        2 => Transient::Bursts,
                        _ => Transient::Game,
                    };
                }
            },
        );
        Self::labelled_tip(ui_, dark, t(Key::AdvOrder), t(Key::OrderTip), |ui_| {
            use pbo_core::order::CoreOrder;
            let labels = [
                t(Key::OrderSequential),
                t(Key::OrderAlternate),
                t(Key::OrderRandom),
                t(Key::OrderPairs),
            ];
            let mut idx = CoreOrder::ALL
                .iter()
                .position(|o| *o == self.plan.order)
                .unwrap_or(0);
            if w::segmented(ui_, dark, &mut idx, &labels) {
                self.plan.order = CoreOrder::ALL[idx];
            }
        });
        Self::labelled(ui_, dark, t(Key::SearchLabel), |ui_| {
            let labels = [t(Key::SearchOff), t(Key::SearchCoarse), t(Key::SearchFine)];
            let mut idx = match self.plan.search {
                Search::Off => 0,
                Search::Coarse => 1,
                Search::Fine => 2,
                // A step of one's own is none of the three.
                Search::Step(_) => usize::MAX,
            };
            if w::segmented(ui_, dark, &mut idx, &labels) {
                self.plan.search = [Search::Off, Search::Coarse, Search::Fine][idx];
            }
        });
        if let Some(step) = self.plan.search.step_size() {
            Self::labelled(ui_, dark, t(Key::AutoStep), |ui_| {
                let chosen =
                    Self::count_stepper(ui_, dark, step, 1, pbo_core::tuner::MAX_STEP as u32);
                if chosen != step {
                    self.plan.search = Search::stepping(chosen);
                }
            });
        }
        if self.plan.search != Search::Off {
            ui_.horizontal(|ui_| {
                ui_.spacing_mut().item_spacing.x = 8.0;
                ui_.add_sized(
                    [100.0, 30.0],
                    egui::Label::new(egui::RichText::new(t(Key::HeadroomLabel)).color(p.muted))
                        .truncate(),
                );
                self.plan.guardband =
                    Self::count_stepper(ui_, dark, self.plan.guardband as u32, 0, 10) as i32;
            });
        }
        // At an error: only for a test of the held values; a search is how a core is found to fail.
        ui_.add_enabled_ui(self.plan.search == Search::Off, |ui_| {
            Self::labelled_tip(ui_, dark, t(Key::AdvOnError), t(Key::OnErrorTip), |ui_| {
                let labels = [
                    t(Key::OnErrorContinue),
                    t(Key::OnErrorSkip),
                    t(Key::OnErrorStop),
                ];
                let mut idx = OnError::ALL
                    .iter()
                    .position(|o| *o == self.plan.on_error)
                    .unwrap_or(1);
                if w::segmented(ui_, dark, &mut idx, &labels) {
                    self.plan.on_error = OnError::ALL[idx];
                }
            });
        });
        w::checkbox(ui_, dark, &mut self.plan.notify, t(Key::AdvSound))
            .on_hover_text(t(Key::SoundTip));
    }

    /// A label on the left in a fixed column, and the control filling the rest of the row.
    fn labelled(
        ui_: &mut egui::Ui,
        dark: bool,
        label: &str,
        add_control: impl FnOnce(&mut egui::Ui),
    ) {
        let p = ui::palette(dark);
        ui_.horizontal(|ui_| {
            ui_.spacing_mut().item_spacing.x = 8.0;
            ui_.add_sized(
                [100.0, 30.0],
                egui::Label::new(egui::RichText::new(label).color(p.muted)).truncate(),
            );
            add_control(ui_);
        });
    }

    /// The same with an explanation that shows when the label is hovered.
    fn labelled_tip(
        ui_: &mut egui::Ui,
        dark: bool,
        label: &str,
        tip: &str,
        add_control: impl FnOnce(&mut egui::Ui),
    ) {
        let p = ui::palette(dark);
        ui_.horizontal(|ui_| {
            ui_.spacing_mut().item_spacing.x = 8.0;
            ui_.add_sized(
                [100.0, 30.0],
                egui::Label::new(egui::RichText::new(label).color(p.muted)).truncate(),
            )
            .on_hover_text(tip);
            add_control(ui_);
        });
    }

    /// One row of choices: a label and a few values to click, with the current one marked. Returns the one that was
    /// clicked, if any.
    fn choice_row(
        ui_: &mut egui::Ui,
        dark: bool,
        label: &str,
        tip: &str,
        options: &[&str],
        current: Option<usize>,
    ) -> Option<usize> {
        let p = ui::palette(dark);
        let mut chosen = None;
        ui_.horizontal(|ui_| {
            ui_.spacing_mut().item_spacing.x = 6.0;
            ui_.add_sized(
                [72.0, 26.0],
                egui::Label::new(egui::RichText::new(label).color(p.muted)).truncate(),
            )
            .on_hover_text(tip);
            let mut idx = current.unwrap_or(usize::MAX);
            if w::segmented(ui_, dark, &mut idx, options) {
                chosen = Some(idx);
            }
        });
        chosen
    }

    /// A stepper with a number in it, for a count: the value and what the person did to it.
    fn count_stepper(ui_: &mut egui::Ui, dark: bool, value: u32, min: u32, max: u32) -> u32 {
        match w::stepper(ui_, dark, &value.to_string(), false, true, 28.0) {
            Stepper::Up => (value + 1).min(max),
            Stepper::Down => value.saturating_sub(1).max(min),
            Stepper::None => value,
        }
    }

    fn advanced_tab(&mut self, ui_: &mut egui::Ui) {
        let dark = self.dark;
        let p = ui::palette(dark);
        let lang = self.lang;
        let t = move |k: Key| text::tr(lang, k);
        let editable = Controls::of(&self.state).can_edit_plan;
        let mut run_plan = false;
        let mut plan_io = 0;
        ui_.spacing_mut().item_spacing.y = 5.0;

        // Two pages: what to test, and how to run it. The same footer under both.
        let mut page = self.adv_page.min(1);
        w::segmented(
            ui_,
            dark,
            &mut page,
            &[t(Key::AdvPageTest), t(Key::AdvPageRun)],
        );
        self.adv_page = page;
        ui_.add_enabled_ui(editable, |ui_| {
            ui_.spacing_mut().item_spacing.y = 5.0;
            if page == 0 {
                self.adv_test_page(ui_);
            } else {
                self.adv_run_page(ui_);
            }
        });

        if !self.plan.then.is_empty() {
            let steps: Vec<String> = self.plan.then.iter().map(|s| s.to_text()).collect();
            ui_.add(
                egui::Label::new(
                    egui::RichText::new(format!("+ {}", steps.join(" → ")))
                        .monospace()
                        .size(ui::TEXT_SMALL)
                        .color(p.muted),
                )
                .truncate(),
            );
        }
        let adjusted = self.plan.adjusted_parts();
        if adjusted > 0 {
            ui_.horizontal(|ui_| {
                ui::chip(ui_, dark, &text::adjusted(lang, adjusted), ChipKind::Accent);
                if w::button(
                    ui_,
                    dark,
                    t(Key::BackToRecommendation),
                    w::ButtonStyle::Quiet,
                    true,
                )
                .clicked()
                {
                    self.plan = Plan::recommended();
                }
            });
        }
        ui_.add_space(2.0);
        // The caption of the command line, and the plan file buttons at the right end of the same line.
        ui_.horizontal(|ui_| {
            ui_.spacing_mut().item_spacing.x = 6.0;
            w::section_label(ui_, dark, t(Key::TheseCommandLine));
            ui_.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui_| {
                if w::button_sized(
                    ui_,
                    dark,
                    t(Key::PlanLoadShort),
                    w::ButtonStyle::Quiet,
                    true,
                    26.0,
                )
                .on_hover_text(t(Key::PlanLoad))
                .clicked()
                {
                    plan_io = 2;
                }
                if w::button_sized(
                    ui_,
                    dark,
                    t(Key::PlanSaveShort),
                    w::ButtonStyle::Quiet,
                    true,
                    26.0,
                )
                .on_hover_text(t(Key::PlanSave))
                .clicked()
                {
                    plan_io = 1;
                }
            });
        });
        // The command line itself only where there is room for it and the button below: at the smallest window the button
        // comes first (the plan file buttons above still save it).
        if ui_.available_height() > 110.0 {
            let mut command = self.plan.command_line();
            ui_.add(
                egui::TextEdit::multiline(&mut command)
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY),
            );
        }
        let can = Controls::of(&self.state).can_start && self.engine_found && self.has_cores();
        if w::big_button(
            ui_,
            dark,
            t(Key::PlanCustomRun),
            w::ButtonStyle::Accent,
            can,
        )
        .clicked()
        {
            run_plan = true;
        }
        if !self.has_cores() {
            paragraph(ui_, t(Key::NoCoreChosen), ui::TEXT_SMALL, p.danger);
        }

        let path = self.store.root.join("plan.toml");
        if plan_io == 1 && !self.is_preview() {
            let ok = crate::store::atomic_write(&path, self.plan.to_toml().as_bytes()).is_ok();
            let msg = if ok {
                t(Key::PlanSaved)
            } else {
                t(Key::PlanLoadFailed)
            };
            self.say_toast(msg);
        } else if plan_io == 2 {
            let loaded = std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|text| {
                    Plan::from_toml(text.trim_start_matches(char::from_u32(0xFEFF).unwrap_or(' ')))
                });
            match loaded {
                Ok(plan) => {
                    self.plan = plan;
                    self.say_toast(t(Key::PlanLoaded));
                }
                Err(_) => self.say_toast(t(Key::PlanLoadFailed)),
            }
        }
        if run_plan {
            let plan = self.plan.clone();
            self.start_with(plan);
        }
    }
}
