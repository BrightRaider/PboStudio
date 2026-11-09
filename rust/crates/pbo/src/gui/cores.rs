//! The left side: one row per core, each a small instrument. The value of the Curve Optimizer is a stepper; beside it a bar
//! shows how deep the core holds within what the processor may be asked for, with a tick where it held at best and one
//! where it failed; then the clock it really runs at and what is known about it. The core under test breathes.
//!
//! The rows take the height they are given: eight cores are comfortable, sixteen are tight, and neither scrolls.

use super::*;
use pbo_ui::widgets::{self as w, Step as Stepper};

/// Where the columns of a row start, from its left edge.
const COL_CHECK: f32 = 14.0;
const COL_NAME: f32 = 44.0;
const COL_STEPPER: f32 = 104.0;
const COL_BAR: f32 = 240.0;

impl App {
    /// Whether a core is part of what is tested: every one when the plan names none and the person has not switched them
    /// all off.
    pub(super) fn core_selected(&self, core: usize) -> bool {
        !self.none_selected && (self.plan.cores.is_empty() || self.plan.cores.contains(&core))
    }

    /// Whether anything can be started: at least one core is chosen.
    pub(super) fn has_cores(&self) -> bool {
        !self.none_selected
    }

    /// The left column: the cores, and under them the drawer with the telemetry or the log when it is open.
    pub(super) fn left_column(&mut self, ui_: &mut egui::Ui) {
        let drawer_h = if self.drawer.is_some() { 196.0 } else { 0.0 };
        let gap = if self.drawer.is_some() { 10.0 } else { 0.0 };
        let total = ui_.available_height();
        let cores_h = (total - drawer_h - gap).max(120.0);
        let width = ui_.available_width();
        ui_.allocate_ui(egui::Vec2::new(width, cores_h), |ui_| {
            ui_.set_min_height(cores_h);
            self.cores_card(ui_, cores_h);
        });
        if self.drawer.is_some() {
            ui_.add_space(gap);
            ui_.allocate_ui(egui::Vec2::new(width, drawer_h), |ui_| {
                ui_.set_min_height(drawer_h);
                self.drawer_card(ui_, drawer_h);
            });
        }
    }

    fn cores_card(&mut self, ui_: &mut egui::Ui, height: f32) {
        let Some(info) = self.info.clone() else {
            self.no_access_card(ui_);
            return;
        };
        let dark = self.dark;
        let p = ui::palette(dark);
        let lang = self.lang;
        let t = move |k: Key| text::tr(lang, k);
        let cores = info.cores.max(1);
        let editable = Controls::of(&self.state).can_edit_plan;
        let running_core = match self.state {
            State::Running { core, .. } => Some(core as usize),
            _ => None,
        };

        let margin = 12.0;
        let head_h = 26.0;
        let gap = 4.0;
        let legend_h = 22.0;
        let rows_room = height - 2.0 * margin - head_h - legend_h - 10.0;
        let row_h = ((rows_room - gap * (cores as f32 - 1.0)) / cores as f32).clamp(26.0, 60.0);

        let surface = ui::Surface {
            margin,
            ..ui::Surface::raised(dark)
        };
        ui::surface(ui_, dark, surface, |ui_| {
            ui_.set_min_height(height - 2.0 * margin);
            ui_.spacing_mut().item_spacing.y = gap;

            // The heads of the columns, with "all cores" over the checkboxes.
            let width = ui_.available_width();
            let (head, _) =
                ui_.allocate_exact_size(egui::Vec2::new(width, head_h), egui::Sense::hover());
            {
                let mut all = !self.none_selected && self.plan.cores.is_empty();
                let check = egui::Rect::from_min_size(
                    egui::Pos2::new(head.left() + 4.0, head.top()),
                    egui::Vec2::new(150.0, head_h),
                );
                ui::place(
                    ui_,
                    check,
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui_| {
                        if w::checkbox(ui_, dark, &mut all, "").changed() {
                            // All cores on, or every one of them off: not "all but the first".
                            self.plan.cores.clear();
                            self.none_selected = !all;
                        }
                        ui_.label(
                            egui::RichText::new(t(Key::ColCore).to_uppercase())
                                .size(ui::TEXT_SMALL - 1.0)
                                .strong()
                                .color(p.muted),
                        );
                    },
                );
                let head_text = |x: f32, text: &str| {
                    ui_.painter().text(
                        egui::Pos2::new(head.left() + x, head.center().y),
                        egui::Align2::LEFT_CENTER,
                        text.to_uppercase(),
                        egui::FontId::new(ui::TEXT_SMALL - 1.0, egui::FontFamily::Proportional),
                        p.muted,
                    );
                };
                head_text(COL_STEPPER, t(Key::CurveOptimizer));
                head_text(COL_BAR, t(Key::ColDepth));
                ui_.painter().text(
                    egui::Pos2::new(head.right() - 6.0, head.center().y),
                    egui::Align2::RIGHT_CENTER,
                    t(Key::LabelClock),
                    egui::FontId::new(ui::TEXT_SMALL - 1.0, egui::FontFamily::Proportional),
                    p.muted,
                );
            }

            for core in 0..cores {
                self.core_row(ui_, core, &info, row_h, editable, running_core);
            }

            // What the bar and its ticks mean.
            ui_.add_space(2.0);
            let (legend, _) =
                ui_.allocate_exact_size(egui::Vec2::new(width, legend_h), egui::Sense::hover());
            let mut x = legend.left() + COL_BAR;
            let y = legend.center().y;
            for (label, color, tick) in [
                (t(Key::LegendHolds), p.accent, false),
                (t(Key::CoreBestHeld), p.ok, true),
                (t(Key::CoreFailedAt), p.danger, true),
            ] {
                let painter = ui_.painter();
                if tick {
                    painter.line_segment(
                        [egui::Pos2::new(x, y - 5.0), egui::Pos2::new(x, y + 5.0)],
                        egui::Stroke::new(2.0_f32, color),
                    );
                } else {
                    painter.rect_filled(
                        egui::Rect::from_center_size(
                            egui::Pos2::new(x + 8.0, y),
                            egui::Vec2::new(16.0, 6.0),
                        ),
                        egui::Rounding::same(3.0),
                        color,
                    );
                }
                let text_x = x + if tick { 8.0 } else { 22.0 };
                let rect = painter.text(
                    egui::Pos2::new(text_x, y),
                    egui::Align2::LEFT_CENTER,
                    label,
                    egui::FontId::new(ui::TEXT_SMALL - 1.0, egui::FontFamily::Proportional),
                    p.muted,
                );
                x = rect.right() + 18.0;
            }
        });
    }

    fn core_row(
        &mut self,
        ui_: &mut egui::Ui,
        core: usize,
        info: &MachineInfo,
        row_h: f32,
        editable: bool,
        running_core: Option<usize>,
    ) {
        let dark = self.dark;
        let p = ui::palette(dark);
        let lang = self.lang;
        let t = move |k: Key| text::tr(lang, k);
        let cores = info.cores;

        let held = self.held.get(core).copied();
        let shown = self.edit.get(&core).copied().or(held);
        let changed = matches!((self.edit.get(&core), held), (Some(e), Some(h)) if *e != h);
        let k = self.knowledge.get(&(core as u32)).copied();
        let clock = self.last_sample.core_clocks.get(core).copied().flatten();
        let active = running_core == Some(core);
        let selected = self.core_selected(core);

        let width = ui_.available_width();
        let (rect, response) =
            ui_.allocate_exact_size(egui::Vec2::new(width, row_h), egui::Sense::hover());
        let compact = width < 600.0;
        let time = ui_.input(|i| i.time) as f32;
        let pulse = if active {
            ui_.ctx().request_repaint_after(Duration::from_millis(40));
            0.5 + 0.5 * (time * 3.2).sin()
        } else {
            0.0
        };
        let hover =
            ui_.ctx()
                .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.14);

        // What is known about the core decides the colour of its edge.
        let status = if active {
            p.accent
        } else if k.is_some_and(|k| k.worst_failed.is_some()) {
            p.danger
        } else if k.is_some_and(|k| k.best_passed.is_some()) {
            p.ok
        } else {
            p.muted.gamma_multiply(0.5)
        };

        // The row itself.
        {
            let painter = ui_.painter();
            if active {
                ui::glow(
                    painter,
                    rect.shrink(3.0),
                    9.0,
                    p.accent.gamma_multiply(0.14 + 0.22 * pulse),
                    14.0,
                );
            }
            let base = ui::Surface {
                radius: 9.0,
                ..ui::Surface::inset(dark)
            };
            let tint = if active {
                base.tinted(p.accent, 0.10 + 0.06 * pulse)
            } else {
                base.tinted(p.accent, 0.04 * hover)
            };
            ui::paint_surface(painter, None, rect, tint, dark);
            let stripe = egui::Rect::from_min_max(
                egui::Pos2::new(rect.left() + 4.0, rect.top() + 6.0),
                egui::Pos2::new(rect.left() + 7.0, rect.bottom() - 6.0),
            );
            painter.rect_filled(
                stripe,
                egui::Rounding::same(1.5),
                status.gamma_multiply(if selected { 1.0 } else { 0.4 }),
            );
        }

        // Left: the checkbox, the name, the stepper.
        let left = egui::Rect::from_min_max(
            egui::Pos2::new(rect.left() + COL_CHECK - 2.0, rect.top()),
            egui::Pos2::new(rect.left() + COL_BAR - 8.0, rect.bottom()),
        );
        let mut step = Stepper::None;
        ui::place(
            ui_,
            left,
            egui::Layout::left_to_right(egui::Align::Center),
            |ui_| {
                ui_.spacing_mut().item_spacing.x = 0.0;
                let mut on = selected;
                if w::checkbox(ui_, dark, &mut on, "").changed() {
                    if self.none_selected {
                        // Nothing was chosen: this one is the first.
                        self.none_selected = false;
                        self.plan.cores = vec![core];
                    } else if self.plan.cores.is_empty() {
                        // All were chosen: this one is taken out.
                        self.plan.cores = (0..cores).filter(|c| *c != core).collect();
                    } else if on {
                        if !self.plan.cores.contains(&core) {
                            self.plan.cores.push(core);
                        }
                    } else {
                        self.plan.cores.retain(|c| *c != core);
                    }
                    if self.plan.cores.len() >= cores {
                        // Every core named is the same as naming none.
                        self.plan.cores.clear();
                    } else if self.plan.cores.is_empty() {
                        // The last one was switched off: nothing is chosen, which is not the same as all.
                        self.none_selected = true;
                    }
                }
                ui_.add_space(COL_NAME - COL_CHECK - 20.0 + 2.0);
                // The best and the second best core get a star (gold and silver), with what it means on hover.
                let star = self.info.as_ref().and_then(|i| {
                    if i.best == Some(core) {
                        Some((p.accent, Key::CoreBest))
                    } else if i.second == Some(core) {
                        Some((p.muted, Key::CoreSecond))
                    } else {
                        None
                    }
                });
                let font = egui::TextStyle::Body.resolve(ui_.style());
                let mut job = egui::text::LayoutJob::default();
                job.append(
                    &format!("{} {core}", text::tr(self.lang, Key::ColCore)),
                    0.0,
                    egui::TextFormat {
                        font_id: font.clone(),
                        color: if selected { p.text } else { p.muted },
                        ..Default::default()
                    },
                );
                // Every row gets the star, invisible where there is none, so that the names stand in one line.
                job.append(
                    " \u{2605}",
                    0.0,
                    egui::TextFormat {
                        font_id: font,
                        color: star.map_or(egui::Color32::TRANSPARENT, |(color, _)| color),
                        ..Default::default()
                    },
                );
                let name =
                    ui_.add_sized([COL_STEPPER - COL_NAME - 4.0, 20.0], egui::Label::new(job));
                if let Some((_, key)) = star {
                    name.on_hover_text(text::tr(self.lang, key));
                }
                match shown {
                    Some(v) => {
                        let height = (row_h - 10.0).clamp(22.0, 30.0);
                        step = w::stepper(ui_, dark, &v.to_string(), changed, editable, height);
                    }
                    None => {
                        ui_.label(t(Key::NotAvailable));
                    }
                }
            },
        );
        if let (Some(v), Some(h)) = (shown, held) {
            let next = match step {
                Stepper::Up => Some((v + 1).min(info.shallowest)),
                Stepper::Down => Some((v - 1).max(info.deepest)),
                Stepper::None => None,
            };
            if let Some(n) = next {
                if n == h {
                    self.edit.remove(&core);
                } else {
                    self.edit.insert(core, n);
                }
            }
        }

        // Right: the clock and the state, filled from the right edge.
        let right = egui::Rect::from_min_max(
            egui::Pos2::new(rect.left() + COL_BAR + 40.0, rect.top()),
            egui::Pos2::new(rect.right() - 8.0, rect.bottom()),
        );
        let (_, used) = ui::place(
            ui_,
            right,
            egui::Layout::right_to_left(egui::Align::Center),
            |ui_| {
                ui_.spacing_mut().item_spacing.x = 8.0;
                // The clock the core really runs at (APERF/MPERF).
                if let Some(mhz) = clock {
                    if !compact {
                        ui_.add_sized(
                            [60.0, 18.0],
                            egui::Label::new(
                                egui::RichText::new(format!("{:.2} GHz", mhz / 1000.0))
                                    .monospace()
                                    .size(ui::TEXT_SMALL)
                                    .color(if active { p.accent } else { p.muted }),
                            ),
                        );
                    }
                }
                if active {
                    ui::chip(ui_, dark, t(Key::StateTestingNow), ChipKind::Accent);
                } else {
                    match k {
                        Some(k) if k.worst_failed.is_some() => {
                            let bad = k.worst_failed.unwrap_or(0);
                            let text = if compact {
                                bad.to_string()
                            } else {
                                format!("{} {bad}", t(Key::CoreFailedAt))
                            };
                            ui::chip(ui_, dark, &text, ChipKind::Danger);
                        }
                        Some(k) if k.best_passed.is_some() => {
                            let good = k.best_passed.unwrap_or(0);
                            let text = if compact {
                                good.to_string()
                            } else {
                                format!("{} {good}", t(Key::CoreBestHeld))
                            };
                            ui::chip(ui_, dark, &text, ChipKind::Ok);
                        }
                        _ => {
                            if !compact {
                                ui::chip(ui_, dark, t(Key::StateReady), ChipKind::Muted);
                            }
                        }
                    }
                    if held.is_some_and(|h| h <= info.deepest) && !compact {
                        ui::chip(ui_, dark, t(Key::AtChipLimit), ChipKind::Accent);
                    }
                }
            },
        );

        // The bar takes what is left in the middle.
        if let Some(v) = shown {
            let x0 = rect.left() + COL_BAR;
            let x1 = (used.left() - 12.0).min(rect.right() - 12.0);
            if x1 - x0 > 36.0 {
                let bar = egui::Rect::from_min_max(
                    egui::Pos2::new(x0, rect.center().y - 8.0),
                    egui::Pos2::new(x1, rect.center().y + 8.0),
                );
                let (best, failed) = k.map_or((None, None), |k| (k.best_passed, k.worst_failed));
                w::depth_bar_at(ui_, dark, bar, v, best, failed, info.deepest, active);
                let tip = ui_.interact(
                    bar.expand2(egui::Vec2::new(0.0, 6.0)),
                    response.id.with("bar"),
                    egui::Sense::hover(),
                );
                tip.on_hover_text(format!(
                    "{v}  ·  {}: {}  ·  {}: {}",
                    t(Key::CoreBestHeld),
                    best.map_or("–".to_string(), |b| b.to_string()),
                    t(Key::CoreFailedAt),
                    failed.map_or("–".to_string(), |b| b.to_string()),
                ));
            }
        }
    }
}
