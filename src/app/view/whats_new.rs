//! What is new in the running version: the notes from the change log built
//! into the app, opened by the message after an update (D47).

use super::*;
use crate::update;

impl App {
    /// Draws the notes while they are open over the full window.
    pub(super) fn whats_new(&mut self, ctx: &egui::Context) {
        if !self.whats_new_open {
            return;
        }
        let notes = update::notes(update::VERSION);
        let room = ctx.content_rect().size();
        let mut close = false;
        let response = egui::Modal::new(egui::Id::unique("whats-new")).show(ctx, |ui| {
            ui.set_width((room.x - 48.0).min(640.0));
            row(ui, |ui| {
                let title = t!("whats-new-title", version = update::VERSION);
                ui.label(title_txt(&title, size::LARGE, None));
                far(ui, |ui| {
                    let tip = hint(&t!("whats-new-close"), "Escape");
                    let size = Vec2::splat(28.0);
                    close = icon_button_ex(ui, Icon::Close, &tip, size, 13.0, None, None).clicked();
                });
            });
            ui.add_space(space::SMALL);
            // As tall as the window allows: the sheet first opens with its
            // top at the middle of the window, where little room is left.
            let tallest = (room.y - 140.0).max(120.0);
            egui::ScrollArea::vertical()
                .max_height(tallest)
                .min_scrolled_height(tallest)
                .show(ui, |ui| {
                    // Room for the scroll bar, which floats over the content.
                    ui.set_max_width(ui.available_width() - 12.0);
                    // The notes are in English: they read left to right in
                    // either language.
                    ui.with_layout(Layout::top_down(Align::Min), |ui| {
                        for note in &notes {
                            note_row(ui, note);
                        }
                    });
                });
        });
        if close || response.should_close() {
            self.whats_new_open = false;
        }
    }
}

/// One note: a dot, then the note on as many lines as it needs.
fn note_row(ui: &mut Ui, note: &str) {
    let color = ui.visuals().text_color();
    let text_w = (ui.available_width() - 18.0).max(0.0);
    ui.horizontal_top(|ui| {
        let line_h = size::BODY * 1.45;
        let (dot, _) = ui.allocate_exact_size(Vec2::new(12.0, line_h), Sense::hover());
        let center = Pos2::new(dot.center().x, dot.top() + line_h / 2.0);
        ui.painter().circle_filled(center, 2.5, color);
        ui.vertical(|ui| {
            ui.set_width(text_w);
            let text = RichText::new(note)
                .size(size::BODY)
                .line_height(Some(line_h))
                .color(color);
            ui.add(egui::Label::new(text).wrap());
        });
    });
    ui.add_space(space::SMALL);
}
