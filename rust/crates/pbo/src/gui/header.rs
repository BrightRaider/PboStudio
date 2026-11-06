//! The top of the window: the mark and the processor on the left, the live readings as ring gauges in the middle, the
//! health of the setup and the language on the right. One row, nothing wraps: what does not fit is left out, least
//! important last.

use super::*;
use pbo_ui::widgets::{self as w, Level};

/// "AMD Ryzen 7 5800X3D 8-Core Processor" as "Ryzen 7 5800X3D".
fn short_cpu(brand: &str) -> String {
    brand
        .split_whitespace()
        .filter(|word| {
            !word.eq_ignore_ascii_case("AMD")
                && !word.eq_ignore_ascii_case("Processor")
                && !word.to_ascii_lowercase().ends_with("-core")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// One gauge's words, ready to draw.
struct Reading {
    label: &'static str,
    value: String,
    sub: String,
    fraction: Option<f32>,
    level: Level,
}

fn reading(
    label: &'static str,
    value: Option<f32>,
    limit: Option<f32>,
    unit: &str,
    decimals: usize,
) -> Reading {
    match (value, limit) {
        (Some(v), Some(l)) if l > 0.0 => Reading {
            label,
            value: format!("{v:.decimals$}"),
            sub: format!("/ {l:.0} {unit}"),
            fraction: Some((v / l).clamp(0.0, 1.0)),
            level: w::level_of(v, l),
        },
        (Some(v), _) => Reading {
            label,
            value: format!("{v:.decimals$}"),
            sub: unit.to_string(),
            fraction: None,
            level: Level::Idle,
        },
        _ => Reading {
            label,
            value: "n/a".to_string(),
            sub: unit.to_string(),
            fraction: None,
            level: Level::Idle,
        },
    }
}

impl App {
    pub(super) fn header(&mut self, ui_: &mut egui::Ui) {
        let dark = self.dark;
        let p = ui::palette(dark);
        let lang = self.lang;
        let t = move |k: Key| text::tr(lang, k);
        let s = self.last_sample.clone();
        let running = matches!(
            self.state,
            State::Preparing | State::Running { .. } | State::Stopping
        );
        let has_access = self.info.is_some();
        let verified = self.info.as_ref().map(|i| i.verified);
        let (health, _) = self.health();
        let brand = self.info.as_ref().map(|i| short_cpu(&i.brand));
        let mut open_setup = false;
        let mut toggle_lang = false;
        let mut toggle_theme = false;

        // Priority order: what the person looks at first comes first, and what does not fit is dropped from the end.
        let readings = [
            Reading {
                label: t(Key::LabelClock),
                value: s.clock_mhz.map_or("n/a".into(), |c| format!("{c:.0}")),
                sub: "MHz".into(),
                fraction: None,
                level: Level::Idle,
            },
            reading(t(Key::LabelTemp), s.temperature, s.temp_limit, "°C", 0),
            reading(t(Key::LabelPower), s.ppt_watts, s.ppt_limit, "W", 0),
            Reading {
                label: t(Key::LabelVcore),
                value: s.core_volts.map_or("n/a".into(), |v| format!("{v:.3}")),
                sub: "V".into(),
                fraction: None,
                level: Level::Idle,
            },
            reading(t(Key::LabelTdc), s.tdc_amps, s.tdc_limit, "A", 0),
            reading(t(Key::LabelEdc), s.edc_amps, s.edc_limit, "A", 0),
        ];

        let surface = ui::Surface {
            margin: 12.0,
            ..ui::Surface::raised(dark)
        };
        ui::surface(ui_, dark, surface, |ui_| {
            let height = 56.0;
            let full = ui_.available_rect_before_wrap();
            let band = egui::Rect::from_min_size(full.min, egui::Vec2::new(full.width(), height));
            let left_w = 196.0;
            let right_w = 236.0;

            // The mark and the processor.
            let left = egui::Rect::from_min_size(band.min, egui::Vec2::new(left_w, height));
            ui::place(
                ui_,
                left,
                egui::Layout::left_to_right(egui::Align::Center),
                |ui_| {
                    ui_.spacing_mut().item_spacing = egui::Vec2::new(10.0, 0.0);
                    w::logo(ui_, dark, 42.0);
                    ui_.vertical(|ui_| {
                        ui_.spacing_mut().item_spacing.y = 2.0;
                        ui_.horizontal(|ui_| {
                            ui_.spacing_mut().item_spacing.x = 5.0;
                            ui_.label(
                                egui::RichText::new("PBO")
                                    .size(19.0)
                                    .strong()
                                    .color(p.accent),
                            );
                            ui_.label(egui::RichText::new("STUDIO").size(19.0).strong());
                        });
                        ui_.horizontal(|ui_| {
                            ui_.spacing_mut().item_spacing.x = 5.0;
                            if let Some(ok) = verified {
                                let (rect, r) = ui_.allocate_exact_size(
                                    egui::Vec2::splat(10.0),
                                    egui::Sense::hover(),
                                );
                                let color = if ok { p.ok } else { p.accent };
                                ui_.painter().circle_filled(rect.center(), 3.5, color);
                                r.on_hover_text(if ok {
                                    t(Key::CheckedOnHardware)
                                } else {
                                    t(Key::NotCheckedHint)
                                });
                            }
                            let name = brand.clone().unwrap_or_default();
                            ui_.add(
                                egui::Label::new(
                                    egui::RichText::new(name)
                                        .size(ui::TEXT_SMALL)
                                        .color(p.muted),
                                )
                                .truncate(),
                            );
                        });
                    });
                },
            );

            // The system on the right: its health, the theme, the language, and what the watcher is doing.
            let right = egui::Rect::from_min_max(
                egui::Pos2::new(band.right() - right_w, band.top()),
                egui::Pos2::new(band.right(), band.bottom()),
            );
            ui::place(
                ui_,
                right,
                egui::Layout::top_down(egui::Align::Max),
                |ui_| {
                    ui_.spacing_mut().item_spacing = egui::Vec2::new(8.0, 4.0);
                    // Each row has the height it needs: a layout given the whole block would take all of it.
                    let row_w = ui_.available_width();
                    ui_.allocate_ui_with_layout(
                        egui::Vec2::new(row_w, 32.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui_| {
                            if w::button(
                                ui_,
                                dark,
                                if lang == Lang::De { "DE" } else { "EN" },
                                w::ButtonStyle::Quiet,
                                true,
                            )
                            .on_hover_text("Ctrl+L")
                            .clicked()
                            {
                                toggle_lang = true;
                            }
                            if w::theme_button(ui_, dark).clicked() {
                                toggle_theme = true;
                            }
                            let label = format!("{} {health}/100", t(Key::SystemTitle));
                            let style = if health >= 100 {
                                w::ButtonStyle::Quiet
                            } else {
                                w::ButtonStyle::Accent
                            };
                            if w::button(ui_, dark, &label, style, true)
                                .on_hover_text(t(Key::HealthTip))
                                .clicked()
                            {
                                open_setup = true;
                            }
                        },
                    );
                    // The watcher: what the program is doing about machine checks right now.
                    let (wlabel, wcolor) = if !has_access {
                        (t(Key::WatchdogNoHelper), p.muted)
                    } else if running {
                        (t(Key::WatchdogActive), p.ok)
                    } else {
                        (t(Key::WatchdogReady), p.ok)
                    };
                    ui_.allocate_ui_with_layout(
                        egui::Vec2::new(row_w, 16.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui_| {
                            ui_.add(
                                egui::Label::new(
                                    egui::RichText::new(wlabel)
                                        .size(ui::TEXT_SMALL)
                                        .color(wcolor),
                                )
                                .truncate(),
                            );
                            let (rect, _) = ui_
                                .allocate_exact_size(egui::Vec2::splat(10.0), egui::Sense::hover());
                            ui_.painter().circle_filled(rect.center(), 3.5, wcolor);
                        },
                    );
                },
            );

            // The gauges, centred between the two, as many as fit.
            let gap = 8.0;
            let room = (band.width() - left_w - right_w - 24.0).max(0.0);
            let mut shown: Vec<&Reading> = Vec::new();
            let mut used = 0.0;
            for r in &readings {
                let width = w::gauge_width(ui_, r.label, &r.value, &r.sub);
                let next = used + width + if shown.is_empty() { 0.0 } else { gap };
                if next > room {
                    break;
                }
                used = next;
                shown.push(r);
            }
            let start = band.left() + left_w + 12.0 + ((room - used) / 2.0).max(0.0);
            let mid = egui::Rect::from_min_size(
                egui::Pos2::new(start, band.top()),
                egui::Vec2::new(used + 2.0, height),
            );
            ui::place(
                ui_,
                mid,
                egui::Layout::left_to_right(egui::Align::Center),
                |ui_| {
                    ui_.spacing_mut().item_spacing.x = gap;
                    for r in shown {
                        w::gauge(ui_, dark, r.label, &r.value, &r.sub, r.fraction, r.level);
                    }
                },
            );

            ui_.allocate_rect(band, egui::Sense::hover());
        });

        if open_setup {
            // The same button opens and closes it.
            self.show_setup = !self.show_setup;
        }
        if toggle_lang {
            self.lang = if self.lang == Lang::De {
                Lang::En
            } else {
                Lang::De
            };
        }
        if toggle_theme {
            self.dark = !self.dark;
            ui::apply(ui_.ctx(), self.dark);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::short_cpu;

    #[test]
    fn a_processor_name_is_cut_to_what_tells_it_apart() {
        assert_eq!(
            short_cpu("AMD Ryzen 7 5800X3D 8-Core Processor"),
            "Ryzen 7 5800X3D"
        );
        assert_eq!(
            short_cpu("AMD Ryzen 9 7950X 16-Core Processor"),
            "Ryzen 9 7950X"
        );
        assert_eq!(short_cpu("Something Else"), "Something Else");
    }
}
