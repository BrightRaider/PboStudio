//! Where everything sits: the header across the top, the action bar across the bottom, the tabs in a column on the
//! right and the cores in the rest. The panels have no fill of their own: the window's gradient shows through, and
//! the cards are what is raised from it.

use super::*;

impl App {
    pub(super) fn layout(&mut self, ctx: &egui::Context) {
        self.shortcuts(ctx);
        let dark = self.dark;
        ui::paint_window_background(ctx, dark);
        let frame = |l: f32, r: f32, t: f32, b: f32| {
            egui::Frame::none().inner_margin(egui::Margin {
                left: l,
                right: r,
                top: t,
                bottom: b,
            })
        };

        egui::TopBottomPanel::top("header")
            .show_separator_line(false)
            .frame(frame(14.0, 14.0, 12.0, 4.0))
            .show(ctx, |ui_| self.header(ui_));
        egui::TopBottomPanel::bottom("bar")
            .show_separator_line(false)
            .exact_height(66.0)
            .frame(frame(14.0, 14.0, 4.0, 12.0))
            .show(ctx, |ui_| self.action_bar(ui_));
        // Without access to the processor there is nothing to choose from: the window shows one thing.
        if self.info.is_some() {
            egui::SidePanel::right("right")
                .show_separator_line(false)
                .exact_width(418.0)
                .resizable(false)
                .frame(frame(0.0, 14.0, 6.0, 6.0))
                .show(ctx, |ui_| self.right_panel(ui_));
        }
        egui::CentralPanel::default()
            .frame(frame(14.0, 10.0, 6.0, 6.0))
            .show(ctx, |ui_| self.left_column(ui_));
        self.windows(ctx);
    }
}
