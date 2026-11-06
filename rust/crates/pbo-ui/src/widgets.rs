//! The widgets of the main screen: ring gauges for the header, the value stepper of a core row, a timeline for the
//! campaign, pills and tiles to choose from, buttons that glow. All painted here, so nothing depends on which symbols a
//! font has, and every border is painted half a point inside its rectangle so that nothing clips it.

use egui::{
    epaint::PathShape, Align, Align2, Color32, FontFamily, FontId, Layout, Pos2, Rect, Response,
    RichText, Rounding, Sense, Shape, Stroke, Ui, Vec2,
};

use crate::{
    chip_color, glow, gradient_rect, paint_surface, palette, ChipKind, Surface, RADIUS_SMALL,
    TEXT_BODY, TEXT_SMALL,
};

/// How alarming a reading is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Idle,
    Ok,
    Warn,
    Danger,
}

fn level_color(dark: bool, level: Level) -> Color32 {
    let p = palette(dark);
    match level {
        Level::Idle => p.muted,
        Level::Ok => p.ok,
        Level::Warn => p.accent,
        Level::Danger => p.danger,
    }
}

/// The level of a reading against its limit: calm below 85 %, a warning from there, danger at the limit.
pub fn level_of(value: f32, limit: f32) -> Level {
    if limit <= 0.0 {
        Level::Idle
    } else if value >= limit * 0.98 {
        Level::Danger
    } else if value >= limit * 0.85 {
        Level::Warn
    } else {
        Level::Ok
    }
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    a.lerp_to_gamma(b, t.clamp(0.0, 1.0))
}

/// `text`, cut with an ellipsis so that it is no wider than `max_width`.
pub fn fit_text(ui: &Ui, text: &str, font: &FontId, max_width: f32) -> String {
    let width = |s: &str| {
        ui.fonts(|f| {
            f.layout_no_wrap(s.to_string(), font.clone(), Color32::WHITE)
                .size()
                .x
        })
    };
    if width(text) <= max_width {
        return text.to_string();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let candidate: String = chars.iter().collect::<String>().trim_end().to_string() + "…";
        if width(&candidate) <= max_width {
            return candidate;
        }
    }
    String::new()
}

fn text_width(ui: &Ui, text: &str, font: &FontId) -> f32 {
    ui.fonts(|f| {
        f.layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE)
            .size()
            .x
    })
}

/// A small caption in capitals above a group: "CURVE OPTIMIZER".
pub fn section_label(ui: &mut Ui, dark: bool, text: &str) {
    let p = palette(dark);
    ui.label(
        RichText::new(text.to_uppercase())
            .size(TEXT_SMALL - 1.0)
            .strong()
            .color(p.muted),
    );
}

// ── Logo ─────────────────────────────────────────────────────────────────

/// The mark: an amber ring with a dot, on a dark disc, with a soft glow. The same picture as the window icon.
pub fn logo(ui: &mut Ui, dark: bool, size: f32) -> Response {
    let p = palette(dark);
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let painter = ui.painter();
    let c = rect.center();
    let r = size / 2.0 - 1.0;
    glow(
        painter,
        Rect::from_center_size(c, Vec2::splat(r * 1.6)),
        r,
        p.accent.gamma_multiply(0.25),
        10.0,
    );
    painter.circle_filled(c, r, Color32::from_rgb(0x16, 0x17, 0x1A));
    painter.circle_stroke(c, r * 0.67, Stroke::new(r * 0.24, p.accent));
    painter.circle_filled(c, r * 0.24, p.accent);
    response
}

// ── Gauge ────────────────────────────────────────────────────────────────

/// How wide a gauge with these words is, to tell beforehand how many fit.
pub fn gauge_width(ui: &Ui, label: &str, value: &str, sub: &str) -> f32 {
    let value_font = FontId::new(15.0, FontFamily::Monospace);
    let small = FontId::new(TEXT_SMALL - 1.0, FontFamily::Proportional);
    let text_w = text_width(ui, value, &value_font)
        .max(text_width(ui, &label.to_uppercase(), &small))
        .max(text_width(ui, sub, &small));
    52.0 + text_w + 14.0
}

/// A reading with a ring: `label` above, `value` in the middle, `sub` below ("of 90 °C"). The ring fills as the
/// reading approaches its limit and takes the colour of how close that is.
pub fn gauge(
    ui: &mut Ui,
    dark: bool,
    label: &str,
    value: &str,
    sub: &str,
    fraction: Option<f32>,
    level: Level,
) -> Response {
    let p = palette(dark);
    let color = level_color(dark, level);
    let value_font = FontId::new(15.0, FontFamily::Monospace);
    let small = FontId::new(TEXT_SMALL - 1.0, FontFamily::Proportional);
    let size = Vec2::new(gauge_width(ui, label, value, sub), 52.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    let animated = ui.ctx().animate_value_with_time(
        response.id.with("fraction"),
        fraction.unwrap_or(0.0).clamp(0.0, 1.0),
        0.35,
    );
    paint_surface(ui.painter(), None, rect, Surface::inset(dark), dark);
    let painter = ui.painter();

    // The ring.
    let center = Pos2::new(rect.left() + 27.0, rect.center().y);
    let radius = 15.5;
    painter.circle_stroke(center, radius, Stroke::new(4.0_f32, p.inset_stroke));
    if fraction.is_some() && animated > 0.004 {
        let start = -std::f32::consts::FRAC_PI_2;
        let sweep = animated * std::f32::consts::TAU;
        let steps = ((sweep / std::f32::consts::TAU) * 56.0).ceil().max(2.0) as usize;
        let points: Vec<Pos2> = (0..=steps)
            .map(|i| {
                let a = start + sweep * i as f32 / steps as f32;
                Pos2::new(center.x + radius * a.cos(), center.y + radius * a.sin())
            })
            .collect();
        painter.add(PathShape::line(points, Stroke::new(4.0_f32, color)));
    } else if fraction.is_none() {
        painter.circle_filled(center, 3.0, color.gamma_multiply(0.8));
    }

    // The words.
    let x = rect.left() + 52.0;
    painter.text(
        Pos2::new(x, rect.top() + 11.0),
        Align2::LEFT_CENTER,
        label.to_uppercase(),
        small.clone(),
        p.muted,
    );
    painter.text(
        Pos2::new(x, rect.center().y + 1.0),
        Align2::LEFT_CENTER,
        value,
        value_font,
        if level == Level::Idle { p.text } else { color },
    );
    painter.text(
        Pos2::new(x, rect.bottom() - 10.0),
        Align2::LEFT_CENTER,
        sub,
        small,
        p.muted,
    );
    response
}

// ── Buttons ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonStyle {
    /// Outlined and quiet.
    Quiet,
    /// Filled with the accent.
    Accent,
    /// Filled with the danger colour.
    Danger,
}

fn fill_colors(
    dark: bool,
    style: ButtonStyle,
    hover: f32,
    enabled: bool,
) -> (Color32, Color32, Color32, Color32) {
    let p = palette(dark);
    match (style, enabled) {
        (ButtonStyle::Quiet, true) => (
            mix(p.card_top, p.accent, 0.10 * hover),
            mix(p.card_bottom, p.accent, 0.06 * hover),
            mix(p.card_stroke, p.accent, 0.35 + 0.65 * hover),
            p.text,
        ),
        (ButtonStyle::Quiet, false) => (
            p.inset_top,
            p.inset_bottom,
            p.inset_stroke,
            p.muted.gamma_multiply(0.6),
        ),
        (ButtonStyle::Accent, true) => (
            mix(p.accent, Color32::WHITE, 0.16 + 0.10 * hover),
            mix(p.accent, Color32::BLACK, 0.16 - 0.06 * hover),
            mix(p.accent, Color32::BLACK, 0.30),
            p.on_accent,
        ),
        (ButtonStyle::Danger, true) => (
            mix(p.danger, Color32::WHITE, 0.14 + 0.10 * hover),
            mix(p.danger, Color32::BLACK, 0.20 - 0.06 * hover),
            mix(p.danger, Color32::BLACK, 0.30),
            Color32::WHITE,
        ),
        (_, false) => (
            p.inset_top,
            p.inset_bottom,
            p.inset_stroke,
            p.muted.gamma_multiply(0.6),
        ),
    }
}

fn paint_button(
    ui: &Ui,
    dark: bool,
    rect: Rect,
    response: &Response,
    style: ButtonStyle,
    enabled: bool,
    radius: f32,
) -> Color32 {
    let p = palette(dark);
    let hover = ui.ctx().animate_bool_with_time(
        response.id.with("hover"),
        enabled && response.hovered(),
        0.14,
    );
    let pressed = enabled && response.is_pointer_button_down_on();
    let (top, bottom, stroke, text) = fill_colors(dark, style, hover, enabled);
    let body = if pressed {
        rect.translate(Vec2::new(0.0, 1.0))
    } else {
        rect
    };
    let painter = ui.painter();
    if enabled && style != ButtonStyle::Quiet {
        let base = if style == ButtonStyle::Accent {
            p.accent
        } else {
            p.danger
        };
        glow(
            painter,
            body.shrink(2.0),
            radius,
            base.gamma_multiply(if pressed { 0.18 } else { 0.22 + 0.18 * hover }),
            12.0 + 10.0 * hover,
        );
    }
    painter.add(gradient_rect(body, radius, top, bottom));
    if enabled && style != ButtonStyle::Quiet {
        painter.add(Shape::line_segment(
            [
                Pos2::new(body.left() + radius, body.top() + 1.5),
                Pos2::new(body.right() - radius, body.top() + 1.5),
            ],
            Stroke::new(1.0_f32, Color32::from_white_alpha(60)),
        ));
    }
    painter.add(Shape::rect_stroke(
        body.shrink(0.5),
        Rounding::same(radius),
        Stroke::new(1.0_f32, stroke),
    ));
    text
}

/// A button as wide as its text. `Quiet` for the many, `Accent` for the one thing a place is for.
pub fn button(ui: &mut Ui, dark: bool, label: &str, style: ButtonStyle, enabled: bool) -> Response {
    button_sized(ui, dark, label, style, enabled, 32.0)
}

/// The same with a height of its own, for the small buttons that sit in a line of text.
pub fn button_sized(
    ui: &mut Ui,
    dark: bool,
    label: &str,
    style: ButtonStyle,
    enabled: bool,
    height: f32,
) -> Response {
    let font = FontId::new(TEXT_BODY, FontFamily::Proportional);
    let w = text_width(ui, label, &font) + 28.0;
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(w.max(64.0), height),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let text = paint_button(ui, dark, rect, &response, style, enabled, RADIUS_SMALL);
    let offset = if enabled && response.is_pointer_button_down_on() {
        1.0
    } else {
        0.0
    };
    ui.painter().text(
        rect.center() + Vec2::new(0.0, offset),
        Align2::CENTER_CENTER,
        label,
        font,
        text,
    );
    if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    }
}

/// The one big button of a panel: as wide as the space, with a glow.
pub fn big_button(
    ui: &mut Ui,
    dark: bool,
    label: &str,
    style: ButtonStyle,
    enabled: bool,
) -> Response {
    let height = 46.0;
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), height),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    // Room round the button for its glow.
    let body = rect.shrink2(Vec2::new(6.0, 3.0));
    let text = paint_button(ui, dark, body, &response, style, enabled, 10.0);
    let offset = if enabled && response.is_pointer_button_down_on() {
        1.0
    } else {
        0.0
    };
    ui.painter().text(
        body.center() + Vec2::new(0.0, offset),
        Align2::CENTER_CENTER,
        label,
        FontId::new(16.0, FontFamily::Proportional),
        text,
    );
    if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    }
}

/// A button that stays pressed: the bottom bar's "Live telemetry" and "Log".
pub fn toggle_button(ui: &mut Ui, dark: bool, label: &str, on: &mut bool) -> Response {
    let p = palette(dark);
    let font = FontId::new(TEXT_BODY, FontFamily::Proportional);
    let w = text_width(ui, label, &font) + 28.0;
    let (rect, mut response) = ui.allocate_exact_size(Vec2::new(w.max(64.0), 32.0), Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let hover =
        ui.ctx()
            .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.14);
    let on_amount = ui
        .ctx()
        .animate_bool_with_time(response.id.with("on"), *on, 0.18);
    let painter = ui.painter();
    painter.add(gradient_rect(
        rect,
        RADIUS_SMALL,
        mix(
            mix(p.card_top, p.accent, 0.10 * hover),
            p.accent,
            0.20 * on_amount,
        ),
        mix(
            mix(p.card_bottom, p.accent, 0.06 * hover),
            p.accent,
            0.10 * on_amount,
        ),
    ));
    painter.add(Shape::rect_stroke(
        rect.shrink(0.5),
        Rounding::same(RADIUS_SMALL),
        Stroke::new(
            1.0_f32,
            mix(
                mix(p.card_stroke, p.accent, 0.35 * hover),
                p.accent,
                on_amount,
            ),
        ),
    ));
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        font,
        mix(p.text, p.accent, on_amount),
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

// ── Stepper ──────────────────────────────────────────────────────────────

/// What the person did to a stepper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    None,
    /// Towards zero.
    Up,
    /// Deeper.
    Down,
}

/// A value box with two arrow buttons: `-20  ^ v`. `pending` paints the value in the accent colour: it has been
/// changed and not yet applied. `height` is the height of the box.
pub fn stepper(
    ui: &mut Ui,
    dark: bool,
    value: &str,
    pending: bool,
    enabled: bool,
    height: f32,
) -> Step {
    let p = palette(dark);
    let size = Vec2::new(122.0, height);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let id = ui
        .id()
        .with(("stepper", rect.min.x as i32, rect.min.y as i32));
    let pending_amount = ui
        .ctx()
        .animate_bool_with_time(id.with("pending"), pending, 0.2);
    let painter = ui.painter();
    if pending_amount > 0.01 {
        glow(
            painter,
            rect.shrink(2.0),
            RADIUS_SMALL,
            p.accent.gamma_multiply(0.22 * pending_amount),
            9.0,
        );
    }
    painter.add(gradient_rect(
        rect,
        RADIUS_SMALL,
        p.inset_top,
        p.inset_bottom,
    ));
    painter.add(Shape::rect_stroke(
        rect.shrink(0.5),
        Rounding::same(RADIUS_SMALL),
        Stroke::new(1.0_f32, mix(p.inset_stroke, p.accent, pending_amount)),
    ));
    painter.text(
        Pos2::new(rect.left() + 12.0, rect.center().y),
        Align2::LEFT_CENTER,
        value,
        FontId::new(15.0, FontFamily::Monospace),
        mix(p.text, p.accent, pending_amount),
    );
    let mut result = Step::None;
    for (i, up) in [true, false].into_iter().enumerate() {
        let b = Rect::from_min_size(
            Pos2::new(rect.right() - 56.0 + 27.0 * i as f32, rect.top() + 2.0),
            Vec2::new(27.0, size.y - 4.0),
        );
        let r = ui.interact(
            b,
            id.with(i),
            if enabled {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        let hover =
            ui.ctx()
                .animate_bool_with_time(id.with(("hover", i)), enabled && r.hovered(), 0.12);
        let painter = ui.painter();
        if hover > 0.01 {
            painter.rect_filled(
                b.shrink(1.0),
                Rounding::same(5.0),
                p.accent.gamma_multiply(0.16 * hover),
            );
        }
        let c = b.center();
        let color = if enabled {
            mix(p.text, p.accent, hover)
        } else {
            p.muted.gamma_multiply(0.45)
        };
        let tri = if up {
            vec![
                Pos2::new(c.x - 4.5, c.y + 2.5),
                Pos2::new(c.x + 4.5, c.y + 2.5),
                Pos2::new(c.x, c.y - 3.5),
            ]
        } else {
            vec![
                Pos2::new(c.x - 4.5, c.y - 2.5),
                Pos2::new(c.x + 4.5, c.y - 2.5),
                Pos2::new(c.x, c.y + 3.5),
            ]
        };
        painter.add(Shape::convex_polygon(tri, color, Stroke::NONE));
        if r.clicked() {
            result = if up { Step::Up } else { Step::Down };
        }
    }
    result
}

// ── Choosing ─────────────────────────────────────────────────────────────

/// A segmented bar whose segments share the width equally. The selected one is marked by a lit shape that slides
/// from one segment to the next. Returns whether the choice changed.
pub fn segmented(ui: &mut Ui, dark: bool, selected: &mut usize, labels: &[&str]) -> bool {
    let p = palette(dark);
    let n = labels.len().max(1);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), Sense::hover());
    let id = ui.id().with((
        "segmented",
        labels.first().copied().unwrap_or(""),
        labels.len(),
        rect.min.y as i32,
    ));
    paint_surface(
        ui.painter(),
        None,
        rect,
        Surface {
            radius: 9.0,
            ..Surface::inset(dark)
        },
        dark,
    );
    let pad = 3.0;
    let inner = rect.shrink(pad);
    let body = FontId::new(TEXT_BODY, FontFamily::Proportional);
    let widths: Vec<f32> = labels.iter().map(|l| text_width(ui, l, &body)).collect();
    let widest = widths.iter().copied().fold(0.0_f32, f32::max);
    let equal = inner.width() / n as f32;
    let small = TEXT_BODY - 1.5;
    let ratio = small / TEXT_BODY;

    // Equal segments, in the body type or a smaller one when the widest word is a little too wide. When even the
    // smaller type does not fit in an equal share, each segment takes what its word needs.
    let proportional = widest * ratio > equal - 10.0;
    let size = if widest > equal - 10.0 {
        small
    } else {
        TEXT_BODY
    };
    let mut edges = vec![inner.left()];
    if proportional {
        let want: Vec<f32> = widths.iter().map(|w| w * ratio + 18.0).collect();
        let scale = inner.width() / want.iter().sum::<f32>().max(1.0);
        for w in &want {
            let last = *edges.last().unwrap_or(&inner.left());
            edges.push(last + w * scale);
        }
    } else {
        for i in 1..=n {
            edges.push(inner.left() + equal * i as f32);
        }
    }
    // The last edge is the right edge, whatever rounding did.
    if let Some(last) = edges.last_mut() {
        *last = inner.right();
    }

    let current = (*selected).min(n - 1);
    let lit_x = ui
        .ctx()
        .animate_value_with_time(id.with("x"), edges[current], 0.16);
    let lit_w =
        ui.ctx()
            .animate_value_with_time(id.with("w"), edges[current + 1] - edges[current], 0.16);
    let lit = Rect::from_min_size(
        Pos2::new(lit_x, inner.top()),
        Vec2::new(lit_w, inner.height()),
    );
    {
        let painter = ui.painter();
        glow(
            painter,
            lit.shrink(1.0),
            7.0,
            p.accent.gamma_multiply(0.25),
            8.0,
        );
        painter.add(gradient_rect(
            lit,
            7.0,
            mix(p.accent, Color32::WHITE, 0.16),
            mix(p.accent, Color32::BLACK, 0.12),
        ));
        painter.add(Shape::rect_stroke(
            lit.shrink(0.5),
            Rounding::same(7.0),
            Stroke::new(1.0_f32, mix(p.accent, Color32::BLACK, 0.3)),
        ));
    }
    let mut changed = false;
    for (i, label) in labels.iter().enumerate() {
        let seg = Rect::from_min_max(
            Pos2::new(edges[i], inner.top()),
            Pos2::new(edges[i + 1], inner.bottom()),
        );
        let r = ui.interact(seg, id.with(i), Sense::click());
        // How much of this segment the lit shape covers: it fades from one word's colour to the next.
        let overlap = (lit.right().min(seg.right()) - lit.left().max(seg.left())).max(0.0);
        let near = (overlap / seg.width().max(1.0)).clamp(0.0, 1.0);
        let hover = ui
            .ctx()
            .animate_bool_with_time(id.with(("hover", i)), r.hovered(), 0.12);
        let color = mix(mix(p.muted, p.text, 0.6 + 0.4 * hover), p.on_accent, near);
        let font = FontId::new(size, FontFamily::Proportional);
        let shown = fit_text(ui, label, &font, seg.width() - 6.0);
        ui.painter()
            .text(seg.center(), Align2::CENTER_CENTER, shown, font, color);
        if r.clicked() && *selected != i {
            *selected = i;
            changed = true;
        }
        if r.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
    }
    changed
}

/// A pill to choose by: one short word or two. `width` is its width; the height is fixed.
pub fn pill(ui: &mut Ui, dark: bool, text: &str, selected: bool, width: f32) -> Response {
    let p = palette(dark);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 30.0), Sense::click());
    let hover =
        ui.ctx()
            .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.12);
    let on = ui
        .ctx()
        .animate_bool_with_time(response.id.with("on"), selected, 0.16);
    let painter = ui.painter();
    if on > 0.01 {
        glow(
            painter,
            rect.shrink(2.0),
            9.0,
            p.accent.gamma_multiply(0.2 * on),
            8.0,
        );
    }
    painter.add(gradient_rect(
        rect,
        9.0,
        mix(
            mix(p.inset_top, p.accent, 0.08 * hover),
            p.accent,
            0.28 * on,
        ),
        mix(
            mix(p.inset_bottom, p.accent, 0.05 * hover),
            p.accent,
            0.14 * on,
        ),
    ));
    painter.add(Shape::rect_stroke(
        rect.shrink(0.5),
        Rounding::same(9.0),
        Stroke::new(
            1.0_f32,
            mix(mix(p.inset_stroke, p.accent, 0.4 * hover), p.accent, on),
        ),
    ));
    let font = FontId::new(TEXT_BODY - 0.5, FontFamily::Proportional);
    let shown = fit_text(ui, text, &font, width - 16.0);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        shown,
        font,
        mix(p.text, p.accent, on),
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A selectable tile of a given size with free contents. The contents are laid out top down inside it.
pub fn tile(
    ui: &mut Ui,
    dark: bool,
    selected: bool,
    size: Vec2,
    add_contents: impl FnOnce(&mut Ui),
) -> Response {
    let p = palette(dark);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let hover =
        ui.ctx()
            .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.14);
    let on = ui
        .ctx()
        .animate_bool_with_time(response.id.with("on"), selected, 0.18);
    let painter = ui.painter();
    if on > 0.01 {
        glow(
            painter,
            rect.shrink(3.0),
            10.0,
            p.accent.gamma_multiply(0.22 * on),
            12.0,
        );
    }
    painter.add(gradient_rect(
        rect,
        10.0,
        mix(
            mix(p.inset_top, p.accent, 0.07 * hover),
            p.accent,
            0.20 * on,
        ),
        mix(
            mix(p.inset_bottom, p.accent, 0.04 * hover),
            p.accent,
            0.08 * on,
        ),
    ));
    painter.add(Shape::rect_stroke(
        rect.shrink(0.5),
        Rounding::same(10.0),
        Stroke::new(
            1.0_f32,
            mix(mix(p.inset_stroke, p.accent, 0.4 * hover), p.accent, on),
        ),
    ));
    let inner = rect.shrink2(Vec2::new(11.0, 9.0));
    crate::place(ui, inner, Layout::top_down(Align::Min), |ui| {
        ui.set_clip_rect(rect.shrink(1.0));
        ui.spacing_mut().item_spacing.y = 2.0;
        add_contents(ui);
    });
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A checkbox in the accent colour: a rounded square, filled amber with a dark tick when on, outlined when off. The label
/// is part of the click target. Returns the response; `checked` is flipped on a click.
pub fn checkbox(ui: &mut Ui, dark: bool, checked: &mut bool, label: &str) -> Response {
    let p = palette(dark);
    let font = FontId::new(TEXT_BODY, FontFamily::Proportional);
    let label_w = if label.is_empty() {
        0.0
    } else {
        text_width(ui, label, &font) + 9.0
    };
    let (rect, mut response) =
        ui.allocate_exact_size(Vec2::new(20.0 + label_w, 22.0), Sense::click());
    if response.clicked() {
        *checked = !*checked;
        response.mark_changed();
    }
    let hover =
        ui.ctx()
            .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.12);
    let on = ui
        .ctx()
        .animate_bool_with_time(response.id.with("on"), *checked, 0.14);
    let painter = ui.painter();
    let box_rect = Rect::from_min_size(
        Pos2::new(rect.left(), rect.center().y - 10.0),
        Vec2::splat(20.0),
    );
    if on > 0.01 {
        glow(
            painter,
            box_rect.shrink(2.0),
            6.0,
            p.accent.gamma_multiply(0.25 * on),
            6.0,
        );
    }
    painter.add(gradient_rect(
        box_rect,
        6.0,
        mix(p.inset_top, mix(p.accent, Color32::WHITE, 0.14), on),
        mix(p.inset_bottom, mix(p.accent, Color32::BLACK, 0.10), on),
    ));
    painter.add(Shape::rect_stroke(
        box_rect.shrink(0.75),
        Rounding::same(6.0),
        Stroke::new(
            1.5_f32,
            mix(
                mix(p.muted.gamma_multiply(0.7), p.accent, hover),
                mix(p.accent, Color32::BLACK, 0.25),
                on,
            ),
        ),
    ));
    if on > 0.05 {
        let c = box_rect.center();
        painter.add(PathShape::line(
            vec![
                Pos2::new(c.x - 5.0, c.y + 0.5),
                Pos2::new(c.x - 1.5, c.y + 4.0),
                Pos2::new(c.x + 5.0, c.y - 4.0),
            ],
            Stroke::new(2.2_f32, p.on_accent.gamma_multiply(on)),
        ));
    }
    if !label.is_empty() {
        painter.text(
            Pos2::new(rect.left() + 29.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            font,
            p.text,
        );
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

// ── Campaign ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepState {
    Done,
    Current,
    Pending,
}

/// The campaign as a vertical timeline: a node (a tick, a lit dot, a ring), the title, and under it the detail.
pub fn timeline(ui: &mut Ui, dark: bool, items: &[(StepState, &str, String)], row_h: f32) {
    let p = palette(dark);
    let time = ui.input(|i| i.time) as f32;
    for (k, (state, title, detail)) in items.iter().enumerate() {
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), Sense::hover());
        let painter = ui.painter();
        let c = Pos2::new(rect.left() + 13.0, rect.top() + 14.0);
        if k + 1 < items.len() {
            painter.line_segment(
                [
                    Pos2::new(c.x, c.y + 11.0),
                    Pos2::new(c.x, rect.bottom() + 3.0),
                ],
                Stroke::new(
                    2.0_f32,
                    if *state == StepState::Done {
                        p.ok.gamma_multiply(0.7)
                    } else {
                        p.inset_stroke
                    },
                ),
            );
        }
        match state {
            StepState::Done => {
                painter.circle_filled(c, 10.0, p.ok.gamma_multiply(0.22));
                painter.circle_stroke(c, 9.5, Stroke::new(1.5_f32, p.ok));
                painter.add(PathShape::line(
                    vec![
                        Pos2::new(c.x - 4.0, c.y + 0.5),
                        Pos2::new(c.x - 1.0, c.y + 3.5),
                        Pos2::new(c.x + 4.5, c.y - 3.0),
                    ],
                    Stroke::new(2.0_f32, p.ok),
                ));
            }
            StepState::Current => {
                let pulse = 0.5 + 0.5 * (time * 2.6).sin();
                glow(
                    painter,
                    Rect::from_center_size(c, Vec2::splat(12.0)),
                    6.0,
                    p.accent.gamma_multiply(0.18 + 0.22 * pulse),
                    12.0,
                );
                painter.circle_filled(c, 10.0, p.accent);
                painter.circle_filled(c, 3.5, p.on_accent);
            }
            StepState::Pending => {
                painter.circle_stroke(c, 9.5, Stroke::new(1.5_f32, p.muted.gamma_multiply(0.7)));
                painter.text(
                    c,
                    Align2::CENTER_CENTER,
                    (k + 1).to_string(),
                    FontId::new(TEXT_SMALL, FontFamily::Proportional),
                    p.muted,
                );
            }
        }
        let font = FontId::new(TEXT_BODY, FontFamily::Proportional);
        let max_w = rect.width() - 38.0;
        let shown = fit_text(ui, title, &font, max_w);
        let painter = ui.painter();
        painter.text(
            Pos2::new(rect.left() + 34.0, rect.top() + 14.0),
            Align2::LEFT_CENTER,
            shown,
            font,
            if *state == StepState::Current {
                p.text
            } else {
                p.muted
            },
        );
        painter.text(
            Pos2::new(rect.left() + 34.0, rect.top() + 32.0),
            Align2::LEFT_CENTER,
            detail,
            FontId::new(TEXT_SMALL, FontFamily::Monospace),
            if *state == StepState::Current {
                p.accent
            } else {
                p.muted.gamma_multiply(0.8)
            },
        );
    }
}

// ── Plots ────────────────────────────────────────────────────────────────

/// A thin line with a soft fill under it, for a reading's recent history.
pub fn sparkline(ui: &mut Ui, dark: bool, height: f32, values: &[f32], color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    paint_surface(
        ui.painter(),
        None,
        rect,
        Surface {
            radius: 6.0,
            ..Surface::inset(dark)
        },
        dark,
    );
    if values.len() < 2 {
        return;
    }
    let painter = ui.painter_at(rect);
    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
    for v in values {
        lo = lo.min(*v);
        hi = hi.max(*v);
    }
    if (hi - lo).abs() < 1e-3 {
        lo -= 1.0;
        hi += 1.0;
    }
    let n = values.len();
    let inner = rect.shrink2(Vec2::new(6.0, 7.0));
    let points: Vec<Pos2> = values
        .iter()
        .enumerate()
        .map(|(k, v)| {
            Pos2::new(
                inner.left() + inner.width() * k as f32 / (n - 1) as f32,
                inner.bottom() - inner.height() * (v - lo) / (hi - lo),
            )
        })
        .collect();
    // The area under the line, fading out downwards.
    let mut mesh = egui::Mesh::default();
    for p in &points {
        mesh.colored_vertex(*p, color.gamma_multiply(0.30));
        mesh.colored_vertex(
            Pos2::new(p.x, rect.bottom() - 1.0),
            color.gamma_multiply(0.0),
        );
    }
    for k in 0..n as u32 - 1 {
        let (a, b) = (2 * k, 2 * k + 2);
        mesh.add_triangle(a, a + 1, b);
        mesh.add_triangle(a + 1, b + 1, b);
    }
    painter.add(Shape::mesh(mesh));
    painter.add(PathShape::line(points, Stroke::new(1.6_f32, color)));
}

/// A bar for one core in a given rectangle: how deep the core holds within what the processor may be asked for, and
/// where it held at best and failed. `depth` is the deepest value allowed (negative); the bar is full at that depth.
/// `active` makes it breathe: the core under test.
#[allow(clippy::too_many_arguments)]
pub fn depth_bar_at(
    ui: &mut Ui,
    dark: bool,
    rect: Rect,
    held: i32,
    best: Option<i32>,
    failed: Option<i32>,
    depth: i32,
    active: bool,
) {
    let p = palette(dark);
    let time = ui.input(|i| i.time) as f32;
    let painter = ui.painter().clone();
    let scale = depth.min(-1) as f32;
    let at = |v: i32| (v as f32 / scale).clamp(0.0, 1.0);
    let track = Rect::from_center_size(
        rect.center(),
        Vec2::new(rect.width(), rect.height().min(10.0)),
    );
    let radius = track.height() / 2.0;
    painter.add(gradient_rect(track, radius, p.inset_top, p.inset_bottom));
    painter.add(Shape::rect_stroke(
        track.shrink(0.5),
        Rounding::same(radius),
        Stroke::new(1.0_f32, p.inset_stroke),
    ));
    let fill_w = (track.width() * at(held)).max(0.0);
    if fill_w > 1.0 {
        let fill = Rect::from_min_size(
            track.min,
            Vec2::new(fill_w.max(track.height()), track.height()),
        )
        .shrink2(Vec2::new(1.0, 1.0));
        let pulse = if active {
            0.5 + 0.5 * (time * 3.2).sin()
        } else {
            0.0
        };
        if active {
            glow(
                &painter,
                fill,
                radius,
                p.accent.gamma_multiply(0.20 + 0.30 * pulse),
                10.0,
            );
        }
        painter.add(gradient_rect(
            fill,
            radius - 1.0,
            mix(p.accent, Color32::WHITE, 0.25 + 0.15 * pulse),
            mix(p.accent, Color32::BLACK, 0.12),
        ));
    }
    for (v, color) in [(best, p.ok), (failed, p.danger)] {
        if let Some(v) = v {
            let x = track.left() + track.width() * at(v);
            painter.line_segment(
                [
                    Pos2::new(x, track.top() - 3.0),
                    Pos2::new(x, track.bottom() + 3.0),
                ],
                Stroke::new(2.0_f32, color),
            );
        }
    }
}

/// A progress bar with a lit fill. `fraction` is 0 to 1.
pub fn progress_bar(ui: &mut Ui, dark: bool, fraction: f32, color: Option<Color32>) {
    let p = palette(dark);
    let color = color.unwrap_or(p.accent);
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 14.0), Sense::hover());
    let f = ui.ctx().animate_value_with_time(
        response.id.with("fraction"),
        fraction.clamp(0.0, 1.0),
        0.4,
    );
    let track = rect.shrink2(Vec2::new(2.0, 2.0));
    let radius = track.height() / 2.0;
    let painter = ui.painter();
    painter.add(gradient_rect(track, radius, p.inset_top, p.inset_bottom));
    painter.add(Shape::rect_stroke(
        track.shrink(0.5),
        Rounding::same(radius),
        Stroke::new(1.0_f32, p.inset_stroke),
    ));
    let w = (track.width() * f).max(if f > 0.0 { track.height() } else { 0.0 });
    if w > 0.0 {
        let fill = Rect::from_min_size(track.min, Vec2::new(w, track.height())).shrink(1.0);
        glow(painter, fill, radius, color.gamma_multiply(0.30), 8.0);
        painter.add(gradient_rect(
            fill,
            radius - 1.0,
            mix(color, Color32::WHITE, 0.25),
            mix(color, Color32::BLACK, 0.12),
        ));
    }
}

/// A coloured dot with a label, for a legend or a state.
pub fn dot(ui: &mut Ui, dark: bool, kind: ChipKind, text: &str) {
    let color = chip_color(dark, kind);
    let p = palette(dark);
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
        ui.painter().circle_filled(rect.center(), 4.0, color);
        ui.label(RichText::new(text).size(TEXT_SMALL).color(p.muted));
    });
}

/// The theme switch: a moon on the dark theme, a sun on the light one. Drawn, not typed, because the font has neither.
pub fn theme_button(ui: &mut Ui, dark: bool) -> Response {
    let p = palette(dark);
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::click());
    let text = paint_button(
        ui,
        dark,
        rect,
        &response,
        ButtonStyle::Quiet,
        true,
        RADIUS_SMALL,
    );
    let c = rect.center();
    let painter = ui.painter();
    if dark {
        // A crescent: a disc with a second disc of the button's own colour taken out of it.
        painter.circle_filled(c, 7.0, text);
        let cut = mix(p.card_top, p.card_bottom, 0.5);
        painter.circle_filled(c + Vec2::new(3.6, -2.8), 6.0, cut);
    } else {
        painter.circle_filled(c, 4.2, text);
        for i in 0..8 {
            let a = std::f32::consts::TAU * i as f32 / 8.0;
            let (s, k) = a.sin_cos();
            painter.line_segment(
                [
                    Pos2::new(c.x + 6.6 * k, c.y + 6.6 * s),
                    Pos2::new(c.x + 9.2 * k, c.y + 9.2 * s),
                ],
                Stroke::new(1.6_f32, text),
            );
        }
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A padlock, drawn: the mark of "this needs rights". `size` is its width.
pub fn lock(ui: &mut Ui, dark: bool, size: f32) -> Response {
    let p = palette(dark);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(size, size * 1.05), Sense::hover());
    let painter = ui.painter();
    let body = Rect::from_min_max(
        Pos2::new(rect.left() + size * 0.10, rect.top() + size * 0.46),
        Pos2::new(rect.right() - size * 0.10, rect.bottom()),
    );
    glow(
        painter,
        body.shrink(3.0),
        8.0,
        p.accent.gamma_multiply(0.25),
        14.0,
    );
    // The shackle.
    let c = Pos2::new(rect.center().x, rect.top() + size * 0.46);
    let r = size * 0.27;
    let points: Vec<Pos2> = (0..=24)
        .map(|i| {
            let a = std::f32::consts::PI + std::f32::consts::PI * i as f32 / 24.0;
            Pos2::new(c.x + r * a.cos(), c.y + r * a.sin() * 1.15)
        })
        .collect();
    painter.add(PathShape::line(
        points,
        Stroke::new(size * 0.09, mix(p.muted, p.text, 0.4)),
    ));
    painter.add(gradient_rect(
        body,
        size * 0.13,
        mix(p.accent, Color32::WHITE, 0.16),
        mix(p.accent, Color32::BLACK, 0.14),
    ));
    painter.add(Shape::rect_stroke(
        body.shrink(0.5),
        Rounding::same(size * 0.13),
        Stroke::new(1.0_f32, mix(p.accent, Color32::BLACK, 0.3)),
    ));
    let k = body.center();
    painter.circle_filled(Pos2::new(k.x, k.y - size * 0.03), size * 0.07, p.on_accent);
    painter.rect_filled(
        Rect::from_center_size(
            Pos2::new(k.x, k.y + size * 0.10),
            Vec2::new(size * 0.06, size * 0.16),
        ),
        Rounding::same(2.0),
        p.on_accent,
    );
    response
}
