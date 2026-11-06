//! The look of PboStudio for egui (ADR-0009): amber accent, soft gradients, glass-edged cards, the system's own Segoe
//! UI. Written against the values in `docs/design.md`; nothing is copied from any other program.
//!
//! Nothing here knows about the processor or the plan. A widget takes what it shows and returns what the person did.
//!
//! Two things are worth knowing when adding to it. A stroke is centred on the edge of its rectangle, so a border that
//! is painted into a clipped painter loses its outer half: every border here is painted on a rectangle shrunk by half a
//! point. And a gradient is a mesh whose vertex colours follow the height, which reproduces a straight vertical
//! gradient exactly because the colour is an affine function of the position.

pub mod icon;
pub mod png;
pub mod widgets;

use egui::{
    epaint::{PathShape, Shadow},
    Align, Color32, FontData, FontDefinitions, FontFamily, FontId, Layout, Margin, Pos2, Rect,
    Response, RichText, Rounding, Sense, Shape, Stroke, TextStyle, Ui, Vec2,
};

/// The colours of one theme. Function colours sit beside the accent and fit into the same language.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    /// The flat colour of the window, for places that cannot take a gradient.
    pub background: Color32,
    /// The window's gradient, top and bottom.
    pub bg_top: Color32,
    pub bg_bottom: Color32,
    /// The flat colour of a card, and its gradient.
    pub card: Color32,
    pub card_top: Color32,
    pub card_bottom: Color32,
    pub card_stroke: Color32,
    /// A surface set into a card (a row, a field, a track): darker at the top, as if recessed.
    pub inset_top: Color32,
    pub inset_bottom: Color32,
    pub inset_stroke: Color32,
    /// The thin light line along the top edge of a raised surface.
    pub sheen: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    /// Text on an accent-coloured button.
    pub on_accent: Color32,
    pub ok: Color32,
    pub danger: Color32,
    pub info: Color32,
}

pub const DARK: Palette = Palette {
    background: Color32::from_rgb(0x13, 0x14, 0x18),
    bg_top: Color32::from_rgb(0x19, 0x1B, 0x21),
    bg_bottom: Color32::from_rgb(0x0F, 0x10, 0x13),
    card: Color32::from_rgb(0x1F, 0x21, 0x26),
    card_top: Color32::from_rgb(0x27, 0x2A, 0x32),
    card_bottom: Color32::from_rgb(0x1C, 0x1E, 0x24),
    card_stroke: Color32::from_rgb(0x32, 0x36, 0x3F),
    inset_top: Color32::from_rgb(0x13, 0x14, 0x18),
    inset_bottom: Color32::from_rgb(0x19, 0x1B, 0x20),
    inset_stroke: Color32::from_rgb(0x2B, 0x2E, 0x36),
    sheen: Color32::from_rgba_premultiplied(18, 18, 18, 18),
    text: Color32::from_rgb(0xEC, 0xED, 0xF0),
    muted: Color32::from_rgb(0x9A, 0xA0, 0xAB),
    accent: Color32::from_rgb(0xF5, 0xA6, 0x23),
    on_accent: Color32::from_rgb(0x1B, 0x14, 0x05),
    ok: Color32::from_rgb(0x4C, 0xAF, 0x7A),
    danger: Color32::from_rgb(0xE5, 0x53, 0x4B),
    info: Color32::from_rgb(0x5B, 0xA3, 0xF5),
};

pub const LIGHT: Palette = Palette {
    background: Color32::from_rgb(0xF1, 0xF3, 0xF6),
    bg_top: Color32::from_rgb(0xF8, 0xF9, 0xFB),
    bg_bottom: Color32::from_rgb(0xE8, 0xEB, 0xF0),
    card: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    card_top: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    card_bottom: Color32::from_rgb(0xF5, 0xF6, 0xF9),
    card_stroke: Color32::from_rgb(0xD9, 0xDE, 0xE6),
    inset_top: Color32::from_rgb(0xEA, 0xED, 0xF2),
    inset_bottom: Color32::from_rgb(0xF4, 0xF5, 0xF8),
    inset_stroke: Color32::from_rgb(0xD3, 0xD8, 0xE1),
    sheen: Color32::from_rgba_premultiplied(120, 120, 120, 120),
    text: Color32::from_rgb(0x1B, 0x1D, 0x22),
    muted: Color32::from_rgb(0x5F, 0x66, 0x72),
    accent: Color32::from_rgb(0xE0, 0x90, 0x0C),
    on_accent: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    ok: Color32::from_rgb(0x2E, 0x8B, 0x57),
    danger: Color32::from_rgb(0xC6, 0x3A, 0x32),
    info: Color32::from_rgb(0x2F, 0x7F, 0xD6),
};

/// Corner radii and text sizes of the design description.
pub const RADIUS_SMALL: f32 = 8.0;
pub const RADIUS_CARD: f32 = 12.0;
pub const TEXT_BODY: f32 = 14.0;
pub const TEXT_SMALL: f32 = 12.5;
pub const TEXT_TITLE: f32 = 22.0;

pub fn palette(dark: bool) -> Palette {
    if dark {
        DARK
    } else {
        LIGHT
    }
}

/// Loads Segoe UI from Windows when it is there; nothing is embedded. The default fonts stay as the fallback.
fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    if let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\segoeui.ttf") {
        fonts
            .font_data
            .insert("segoe".into(), FontData::from_owned(bytes));
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "segoe".into());
    }
    ctx.set_fonts(fonts);
}

/// Sets colours, sizes and fonts. Call once at start and again when the theme changes.
pub fn apply(ctx: &egui::Context, dark: bool) {
    let p = palette(dark);
    install_fonts(ctx);

    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.override_text_color = Some(p.text);
    visuals.panel_fill = p.background;
    visuals.window_fill = p.card;
    visuals.extreme_bg_color = p.inset_top;
    visuals.faint_bg_color = p.card;
    visuals.window_stroke = Stroke::new(1.0_f32, p.card_stroke);
    visuals.window_rounding = Rounding::same(RADIUS_CARD);
    visuals.window_shadow = Shadow {
        offset: Vec2::new(0.0, 8.0),
        blur: 28.0,
        spread: 0.0,
        color: Color32::from_black_alpha(if dark { 120 } else { 40 }),
    };
    visuals.popup_shadow = visuals.window_shadow;
    visuals.selection.bg_fill = p.accent.gamma_multiply(0.35);
    visuals.selection.stroke = Stroke::new(1.0_f32, p.accent);
    visuals.hyperlink_color = p.accent;
    visuals.slider_trailing_fill = true;
    let radius = Rounding::same(RADIUS_SMALL);
    for w in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        w.rounding = radius;
    }
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.card_stroke);
    visuals.widgets.inactive.bg_fill = p.inset_stroke;
    visuals.widgets.inactive.weak_bg_fill = p.inset_bottom;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.inset_stroke);
    visuals.widgets.hovered.bg_fill = p.card_stroke;
    visuals.widgets.hovered.weak_bg_fill = p.card_top;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, p.accent);
    visuals.widgets.active.bg_fill = p.card_stroke;
    visuals.widgets.active.weak_bg_fill = p.card_top;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, p.accent);
    // The chosen theme holds whatever the Windows theme is: it is set for that theme and made the preference.
    let theme = if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    ctx.set_theme(theme);
    ctx.set_visuals_of(theme, visuals);

    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(TEXT_TITLE, FontFamily::Proportional),
        ),
        (
            TextStyle::Body,
            FontId::new(TEXT_BODY, FontFamily::Proportional),
        ),
        (
            TextStyle::Button,
            FontId::new(TEXT_BODY, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(TEXT_SMALL, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(TEXT_SMALL, FontFamily::Monospace),
        ),
    ]
    .into();
    style.spacing.item_spacing = Vec2::new(10.0, 8.0);
    style.spacing.button_padding = Vec2::new(14.0, 7.0);
    // A program is not a document: text in it is not picked up by the mouse, and a click on a label is a click on
    // whatever is behind it.
    style.interaction.selectable_labels = false;
    ctx.set_style_of(theme, style);
}

// ── Surfaces ──────────────────────────────────────────────────────────────

/// The colour a vertical gradient has at `y`.
fn at(y: f32, rect: Rect, top: Color32, bottom: Color32) -> Color32 {
    let t = if rect.height() > 0.0 {
        ((y - rect.top()) / rect.height()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    top.lerp_to_gamma(bottom, t)
}

/// The corner points of a rounded rectangle, clockwise from the top-left corner's start, as a convex polygon.
fn rounded_outline(rect: Rect, radius: f32) -> Vec<Pos2> {
    let r = radius
        .min(rect.width() / 2.0)
        .min(rect.height() / 2.0)
        .max(0.0);
    if r < 0.5 {
        return vec![
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
        ];
    }
    let steps = 7;
    let mut points = Vec::with_capacity(4 * (steps + 1));
    let corners = [
        (
            Pos2::new(rect.left() + r, rect.top() + r),
            std::f32::consts::PI,
        ),
        (
            Pos2::new(rect.right() - r, rect.top() + r),
            1.5 * std::f32::consts::PI,
        ),
        (Pos2::new(rect.right() - r, rect.bottom() - r), 0.0),
        (
            Pos2::new(rect.left() + r, rect.bottom() - r),
            0.5 * std::f32::consts::PI,
        ),
    ];
    for (center, start) in corners {
        for i in 0..=steps {
            let a = start + 0.5 * std::f32::consts::PI * i as f32 / steps as f32;
            points.push(Pos2::new(center.x + r * a.cos(), center.y + r * a.sin()));
        }
    }
    points
}

/// A rounded rectangle filled with a vertical gradient.
pub fn gradient_rect(rect: Rect, radius: f32, top: Color32, bottom: Color32) -> Shape {
    let outline = rounded_outline(rect, radius);
    let mut mesh = egui::Mesh::default();
    for p in &outline {
        mesh.colored_vertex(*p, at(p.y, rect, top, bottom));
    }
    for i in 1..outline.len() as u32 - 1 {
        mesh.add_triangle(0, i, i + 1);
    }
    Shape::mesh(mesh)
}

/// Paints the window's own gradient behind everything. Call once per frame, before the panels.
pub fn paint_window_background(ctx: &egui::Context, dark: bool) {
    let p = palette(dark);
    let rect = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());
    painter.add(gradient_rect(rect, 0.0, p.bg_top, p.bg_bottom));
}

/// A soft glow around a rectangle: a shadow with no offset in a colour.
pub fn glow(painter: &egui::Painter, rect: Rect, radius: f32, color: Color32, blur: f32) {
    painter.add(
        Shadow {
            offset: Vec2::ZERO,
            blur,
            spread: 0.0,
            color,
        }
        .as_shape(rect, Rounding::same(radius)),
    );
}

/// What a raised or recessed surface looks like.
#[derive(Debug, Clone, Copy)]
pub struct Surface {
    pub top: Color32,
    pub bottom: Color32,
    pub stroke: Color32,
    pub radius: f32,
    pub margin: f32,
    /// A thin light line along the top edge.
    pub sheen: Option<Color32>,
    /// A soft drop shadow underneath.
    pub shadow: bool,
}

impl Surface {
    /// A card: raised, with a light top edge and a shadow.
    pub fn raised(dark: bool) -> Surface {
        let p = palette(dark);
        Surface {
            top: p.card_top,
            bottom: p.card_bottom,
            stroke: p.card_stroke,
            radius: RADIUS_CARD,
            margin: 14.0,
            sheen: Some(p.sheen),
            shadow: true,
        }
    }

    /// A field or a row: set into the card.
    pub fn inset(dark: bool) -> Surface {
        let p = palette(dark);
        Surface {
            top: p.inset_top,
            bottom: p.inset_bottom,
            stroke: p.inset_stroke,
            radius: RADIUS_SMALL,
            margin: 10.0,
            sheen: None,
            shadow: false,
        }
    }

    pub fn tinted(self, color: Color32, strength: f32) -> Surface {
        Surface {
            top: self.top.lerp_to_gamma(color, strength * 1.3),
            bottom: self.bottom.lerp_to_gamma(color, strength),
            stroke: self.stroke.lerp_to_gamma(color, (strength * 5.0).min(0.8)),
            ..self
        }
    }
}

/// Paints a surface behind the shape of `rect` at an earlier position in the draw order.
pub fn paint_surface(
    painter: &egui::Painter,
    at_idx: Option<egui::layers::ShapeIdx>,
    rect: Rect,
    s: Surface,
    dark: bool,
) {
    let mut shapes = Vec::new();
    if s.shadow {
        shapes.push(
            Shadow {
                offset: Vec2::new(0.0, 5.0),
                blur: 18.0,
                spread: 0.0,
                color: Color32::from_black_alpha(if dark { 70 } else { 22 }),
            }
            .as_shape(rect, Rounding::same(s.radius))
            .into(),
        );
    }
    shapes.push(gradient_rect(rect, s.radius, s.top, s.bottom));
    shapes.push(Shape::rect_stroke(
        rect.shrink(0.5),
        Rounding::same(s.radius),
        Stroke::new(1.0_f32, s.stroke),
    ));
    if let Some(sheen) = s.sheen {
        let y = rect.top() + 1.5;
        shapes.push(Shape::line_segment(
            [
                Pos2::new(rect.left() + s.radius, y),
                Pos2::new(rect.right() - s.radius, y),
            ],
            Stroke::new(1.0_f32, sheen),
        ));
    }
    match at_idx {
        Some(idx) => painter.set(idx, Shape::Vec(shapes)),
        None => {
            painter.add(Shape::Vec(shapes));
        }
    }
}

/// A surface around contents. The surface is painted behind what is drawn in it, whose size it takes.
pub fn surface<R>(
    ui: &mut Ui,
    dark: bool,
    s: Surface,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    let idx = ui.painter().add(Shape::Noop);
    let out = egui::Frame::none()
        .inner_margin(Margin::same(s.margin))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add_contents(ui)
        });
    paint_surface(ui.painter(), Some(idx), out.response.rect, s, dark);
    out.inner
}

/// Lays contents out in a rectangle that has already been allocated, and returns what they used. Unlike a scope, it does
/// not move the cursor of the parent: a scope would, and back to the end of what it used, which puts the next thing
/// into the middle of the rectangle.
pub fn place<R>(
    ui: &mut Ui,
    rect: Rect,
    layout: Layout,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> (R, Rect) {
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(layout));
    let out = add_contents(&mut child);
    let used = child.min_rect();
    (out, used)
}

/// A card: a raised surface with an optional title, holding rows.
pub fn card<R>(
    ui: &mut Ui,
    dark: bool,
    title: Option<&str>,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    surface(ui, dark, Surface::raised(dark), |ui| {
        if let Some(t) = title {
            ui.label(RichText::new(t).size(TEXT_BODY + 1.0).strong());
            ui.add_space(4.0);
        }
        add_contents(ui)
    })
}

/// A row: the label on the left, its explanation as a tooltip, the control on the right.
pub fn row<R>(
    ui: &mut Ui,
    label: &str,
    hint: Option<&str>,
    add_control: impl FnOnce(&mut Ui) -> R,
) -> R {
    // A fixed height, so that the label and a taller control on the right sit on one centre line.
    let width = ui.available_width();
    ui.allocate_ui_with_layout(
        Vec2::new(width, 34.0),
        Layout::left_to_right(Align::Center),
        |ui| {
            let l = ui.label(label);
            if let Some(h) = hint {
                l.on_hover_text(h);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), add_control)
                .inner
        },
    )
    .inner
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChipKind {
    Ok,
    Danger,
    Info,
    Muted,
    Accent,
}

pub fn chip_color(dark: bool, kind: ChipKind) -> Color32 {
    let p = palette(dark);
    match kind {
        ChipKind::Ok => p.ok,
        ChipKind::Danger => p.danger,
        ChipKind::Info => p.info,
        ChipKind::Muted => p.muted,
        ChipKind::Accent => p.accent,
    }
}

/// Items in the order they will be seen. Inside a row the control sits at the right edge and a layout there fills
/// from the right, so a list that should read left to right is walked backwards.
pub fn in_display_order<T>(ui: &Ui, mut items: Vec<T>) -> Vec<T> {
    if ui.layout().prefer_right_to_left() {
        items.reverse();
    }
    items
}

/// A small rounded label: a hint, a state. Sized from its text, so it is as tall as a line of text however much
/// room it is given.
pub fn chip(ui: &mut Ui, dark: bool, text: &str, kind: ChipKind) -> Response {
    let color = chip_color(dark, kind);
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        FontId::new(TEXT_SMALL, FontFamily::Proportional),
        color,
    );
    let size = Vec2::new(galley.size().x + 20.0, galley.size().y + 6.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    let radius = rect.height() / 2.0;
    let painter = ui.painter();
    painter.add(gradient_rect(
        rect,
        radius,
        color.gamma_multiply(0.26),
        color.gamma_multiply(0.10),
    ));
    painter.add(Shape::rect_stroke(
        rect.shrink(0.5),
        Rounding::same(radius),
        Stroke::new(1.0_f32, color.gamma_multiply(0.55)),
    ));
    painter.galley(
        Pos2::new(
            rect.center().x - galley.size().x / 2.0,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        color,
    );
    response
}

/// The main button of a screen, in the accent colour, as wide as its text.
pub fn primary_button(ui: &mut Ui, dark: bool, text: &str, enabled: bool) -> Response {
    widgets::button(ui, dark, text, widgets::ButtonStyle::Accent, enabled)
}

/// One line of a graph.
pub struct Series<'a> {
    pub name: &'a str,
    pub values: &'a [f32],
    pub color: Color32,
    pub unit: &'a str,
}

/// A small line graph that draws its own axes: the newest value on the right, the range on the left. A series with
/// no values draws nothing. This is the telemetry graph; it takes what it shows and returns nothing.
pub fn graph(ui: &mut Ui, dark: bool, height: f32, series: &[Series]) {
    let p = palette(dark);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    paint_surface(ui.painter(), None, rect, Surface::inset(dark), dark);
    let painter = ui.painter_at(rect);
    let inner = rect.shrink2(Vec2::new(10.0, 8.0));
    for (i, s) in series.iter().enumerate() {
        if s.values.len() < 2 {
            continue;
        }
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        for v in s.values {
            lo = lo.min(*v);
            hi = hi.max(*v);
        }
        if (hi - lo).abs() < 1e-3 {
            lo -= 1.0;
            hi += 1.0;
        }
        let n = s.values.len();
        let points: Vec<Pos2> = s
            .values
            .iter()
            .enumerate()
            .map(|(k, v)| {
                let x = inner.left() + inner.width() * k as f32 / (n - 1) as f32;
                let y = inner.bottom() - inner.height() * (v - lo) / (hi - lo);
                Pos2::new(x, y)
            })
            .collect();
        painter.add(PathShape::line(points, Stroke::new(1.6_f32, s.color)));
        let last = s.values[n - 1];
        painter.text(
            Pos2::new(inner.left() + 4.0 + 150.0 * i as f32, rect.top() + 4.0),
            egui::Align2::LEFT_TOP,
            format!("{} {last:.1} {}", s.name, s.unit),
            FontId::new(TEXT_SMALL, FontFamily::Proportional),
            s.color,
        );
    }
    let _ = p;
}

/// A plain value with its label, for the readings: "Temperature  77.4 °C".
pub fn reading(ui: &mut Ui, dark: bool, label: &str, value: &str) {
    let p = palette(dark);
    ui.vertical(|ui| {
        ui.label(RichText::new(label).size(TEXT_SMALL).color(p.muted));
        ui.label(RichText::new(value).size(TEXT_TITLE - 4.0).strong());
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_accent_is_amber_in_both_themes() {
        assert_eq!(DARK.accent, Color32::from_rgb(0xF5, 0xA6, 0x23));
        assert_eq!(LIGHT.accent, Color32::from_rgb(0xE0, 0x90, 0x0C));
    }

    #[test]
    fn text_is_readable_on_the_surfaces() {
        // A rough contrast check: the text is much lighter or darker than the card in both themes.
        let lum =
            |c: Color32| 0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32;
        assert!((lum(DARK.text) - lum(DARK.card)).abs() > 150.0);
        assert!((lum(LIGHT.text) - lum(LIGHT.card)).abs() > 150.0);
        assert!((lum(DARK.on_accent) - lum(DARK.accent)).abs() > 100.0);
        assert!((lum(LIGHT.on_accent) - lum(LIGHT.accent)).abs() > 40.0);
    }

    #[test]
    fn a_rounded_outline_is_convex_and_stays_inside_its_rectangle() {
        let rect = Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(100.0, 50.0));
        let points = rounded_outline(rect, 12.0);
        assert!(points.len() >= 28);
        for p in &points {
            assert!(rect.expand(0.01).contains(*p), "{p:?}");
        }
        // Convex: every turn goes the same way.
        let n = points.len();
        let mut sign = 0.0_f32;
        for i in 0..n {
            let (a, b, c) = (points[i], points[(i + 1) % n], points[(i + 2) % n]);
            let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
            if cross.abs() > 1e-4 {
                if sign == 0.0 {
                    sign = cross.signum();
                }
                assert_eq!(cross.signum(), sign, "turn {i}");
            }
        }
    }

    #[test]
    fn a_gradient_has_its_top_colour_at_the_top_and_its_bottom_colour_at_the_bottom() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(10.0, 40.0));
        let (a, b) = (Color32::from_rgb(200, 0, 0), Color32::from_rgb(0, 0, 200));
        assert_eq!(at(0.0, rect, a, b), a);
        assert_eq!(at(40.0, rect, a, b), b);
        assert_eq!(at(-5.0, rect, a, b), a);
    }
}
