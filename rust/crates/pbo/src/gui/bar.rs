//! The bottom of the window: the bar with what is done to the values (apply, to the limit, back to the start values, the
//! BIOS list, on every Windows start) and the two switches for the drawer, and the drawer itself with the live readings
//! or the log.

use super::*;
use pbo_ui::widgets as w;

impl App {
    pub(super) fn action_bar(&mut self, ui_: &mut egui::Ui) {
        let dark = self.dark;
        let p = ui::palette(dark);
        let lang = self.lang;
        let t = move |k: Key| text::tr(lang, k);
        let editable = Controls::of(&self.state).can_edit_plan;
        let pending = self.edit.len();
        let busy = self.setup_busy.is_some();
        let logon = self.logon_task;
        let have_cores = self.info.is_some();
        let mut apply = false;
        let mut all_deep = false;
        let mut from_start = false;
        let mut bios = false;
        let mut every_start: Option<bool> = None;

        let surface = ui::Surface {
            margin: 9.0,
            ..ui::Surface::raised(dark)
        };
        ui::surface(ui_, dark, surface, |ui_| {
            ui_.horizontal(|ui_| {
                ui_.spacing_mut().item_spacing.x = 8.0;
                let apply_label = if pending > 0 {
                    format!("{} · {}", t(Key::ApplyLive), text::pending(lang, pending))
                } else {
                    t(Key::ApplyLive).to_string()
                };
                let style = if pending > 0 {
                    w::ButtonStyle::Accent
                } else {
                    w::ButtonStyle::Quiet
                };
                if w::button(ui_, dark, &apply_label, style, editable && pending > 0)
                    .on_hover_text(t(Key::ApplyLiveHint))
                    .clicked()
                {
                    apply = true;
                }
                if w::button(
                    ui_,
                    dark,
                    t(Key::AllToDeepest),
                    w::ButtonStyle::Quiet,
                    editable && have_cores,
                )
                .clicked()
                {
                    all_deep = true;
                }
                if w::button(
                    ui_,
                    dark,
                    t(Key::BiosValues),
                    w::ButtonStyle::Quiet,
                    editable && have_cores,
                )
                .clicked()
                {
                    from_start = true;
                }
                ui_.add_space(6.0);
                let (sep, _) =
                    ui_.allocate_exact_size(egui::Vec2::new(1.0, 22.0), egui::Sense::hover());
                ui_.painter().rect_filled(sep, 0.0, p.card_stroke);
                ui_.add_space(6.0);
                if w::button(
                    ui_,
                    dark,
                    t(Key::BiosList),
                    w::ButtonStyle::Quiet,
                    have_cores,
                )
                .on_hover_text(t(Key::BiosListHint))
                .clicked()
                {
                    bios = true;
                }
                let mut on = logon;
                if w::checkbox(ui_, dark, &mut on, t(Key::EveryStart))
                    .on_hover_text(t(Key::EveryStartHint))
                    .changed()
                    && !busy
                {
                    every_start = Some(on);
                }

                // The shortcut hint is only drawn when there is room for it beside the two buttons: at the smallest window it
                // would run into the checkbox label.
                let room_for_hint = ui_.available_width() > 250.0;
                ui_.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui_| {
                    let mut log = self.drawer == Some(1);
                    if w::toggle_button(ui_, dark, t(Key::LogButton), &mut log).clicked() {
                        self.drawer = if log { Some(1) } else { None };
                        if log {
                            self.refresh_log_tail();
                        }
                    }
                    let mut telemetry = self.drawer == Some(0);
                    if w::toggle_button(ui_, dark, t(Key::LiveTelemetry), &mut telemetry).clicked()
                    {
                        self.drawer = if telemetry { Some(0) } else { None };
                    }
                    if room_for_hint {
                        ui_.label(egui::RichText::new("?").strong().color(p.muted))
                            .on_hover_text(t(Key::Shortcuts));
                    }
                });
            });
        });

        if let Some(info) = self.info.clone() {
            if all_deep {
                for core in 0..info.cores {
                    let selected = self.core_selected(core);
                    if selected && self.held.get(core).is_some_and(|h| *h != info.deepest) {
                        self.edit.insert(core, info.deepest);
                    }
                }
            }
            if from_start {
                self.edit.clear();
                for (core, v) in self.start_values.clone().into_iter().enumerate() {
                    if self.held.get(core).is_some_and(|h| *h != v) {
                        self.edit.insert(core, v);
                    }
                }
            }
        }
        if apply {
            self.apply_pending();
        }
        if bios {
            self.show_bios = !self.show_bios;
        }
        if let Some(on) = every_start {
            self.set_every_start(on);
        }
    }

    pub(super) fn drawer_card(&mut self, ui_: &mut egui::Ui, height: f32) {
        let dark = self.dark;
        let p = ui::palette(dark);
        let lang = self.lang;
        let t = move |k: Key| text::tr(lang, k);
        let surface = ui::Surface {
            margin: 12.0,
            ..ui::Surface::raised(dark)
        };
        ui::surface(ui_, dark, surface, |ui_| {
            ui_.set_min_height(height - 24.0);
            match self.drawer {
                Some(0) => {
                    let s = self.last_sample.clone();
                    let series: [(&str, String, Vec<f32>, egui::Color32); 3] = [
                        (
                            t(Key::Temperature),
                            s.temperature.map_or("n/a".into(), |v| format!("{v:.1} °C")),
                            self.temp.iter().copied().collect(),
                            p.danger,
                        ),
                        (
                            t(Key::Power),
                            s.ppt_watts.map_or("n/a".into(), |v| format!("{v:.1} W")),
                            self.ppt.iter().copied().collect(),
                            p.accent,
                        ),
                        (
                            t(Key::CoreVoltage),
                            s.core_volts.map_or("n/a".into(), |v| format!("{v:.3} V")),
                            self.vcore.iter().copied().collect(),
                            p.info,
                        ),
                    ];
                    ui_.columns(3, |cols| {
                        for (i, (label, value, values, color)) in series.iter().enumerate() {
                            w::section_label(&mut cols[i], dark, label);
                            cols[i].label(egui::RichText::new(value).size(22.0).strong());
                            w::sparkline(&mut cols[i], dark, 62.0, values, *color);
                        }
                    });
                    let vids: Vec<String> = s
                        .core_vids
                        .iter()
                        .enumerate()
                        .map(|(c, v)| match v {
                            Some(x) => format!("{c}: {x:.3}"),
                            None => format!("{c}: n/a"),
                        })
                        .collect();
                    if !vids.is_empty() {
                        ui_.add_space(4.0);
                        ui_.add(
                            egui::Label::new(
                                egui::RichText::new(format!("VID  {}", vids.join("   ")))
                                    .monospace()
                                    .size(ui::TEXT_SMALL - 1.0)
                                    .color(p.muted),
                            )
                            .truncate(),
                        );
                    }
                }
                _ => {
                    let lines: Vec<String> = if self.log_tail.is_empty() {
                        self.log.clone()
                    } else {
                        self.log_tail.clone()
                    };
                    if lines.is_empty() {
                        ui_.label(t(Key::LogEmpty));
                        return;
                    }
                    let rows = lines.len();
                    egui::ScrollArea::vertical()
                        .stick_to_bottom(true)
                        .auto_shrink([false, false])
                        .max_height(height - 28.0)
                        .show_rows(ui_, ui::TEXT_SMALL + 4.0, rows, |ui_, range| {
                            for i in range {
                                ui_.add(
                                    egui::Label::new(
                                        egui::RichText::new(&lines[i])
                                            .monospace()
                                            .size(ui::TEXT_SMALL),
                                    )
                                    .truncate(),
                                );
                            }
                        });
                }
            }
        });
    }
}
