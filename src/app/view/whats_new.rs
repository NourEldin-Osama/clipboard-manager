//! What is new in the running version: the notes from the change log built
//! into the app, opened by the message after an update (D47).

use super::*;
use crate::update;

/// How wide the sheet's content is at most, for lines of about 70 letters.
const SHEET_WIDTH: f32 = 520.0;

/// The room inside the sheet's edge, the same on every side.
const SHEET_MARGIN: f32 = space::HUGE;

/// The room the sheet keeps from the window's edges.
const WINDOW_ROOM: f32 = 48.0;

/// How tall the list of notes is at most.
const LIST_HEIGHT: f32 = 480.0;

/// How tall the title's row and the Close button's row are.
const BAR_HEIGHT: f32 = 36.0;

/// How big the close button beside the title is, and its cross.
const CLOSE_BUTTON: f32 = 36.0;
const CLOSE_CROSS: f32 = 18.0;

/// How wide the place of a note's dot is.
const DOT_PLACE: f32 = 12.0;

impl App {
    /// Draws the notes while they are open over the full window.
    pub(super) fn whats_new(&mut self, ctx: &egui::Context) {
        if !self.whats_new_open {
            return;
        }
        let response = sheet(ctx, &update::notes(update::VERSION));
        let (_, clicked) = response.inner;
        if clicked || response.should_close() {
            self.whats_new_open = false;
        }
    }
}

/// The sheet of `notes` over the window: the title with the close button,
/// the notes in a list that scrolls, and a Close button, with the same room
/// inside the edge on every side. Its content holds where the title, notes
/// and Close button are together, and whether a close button was clicked.
fn sheet(ctx: &egui::Context, notes: &[String]) -> egui::ModalResponse<(Rect, bool)> {
    let frame = egui::Frame::popup(&ctx.global_style())
        .inner_margin(egui::Margin::same(SHEET_MARGIN as i8));
    // The room around the sheet, its edge and its margin, on each side.
    let side = WINDOW_ROOM + frame.stroke.width + SHEET_MARGIN;
    let room = ctx.content_rect().size();
    let width = (room.x - 2.0 * side).clamp(200.0, SHEET_WIDTH);
    // The list takes what is left after those, the two bars and the gaps
    // beside them.
    let around = 2.0 * (side + BAR_HEIGHT + space::MEDIUM);
    let list_height = (room.y - around).clamp(120.0, LIST_HEIGHT);
    egui::Modal::new(egui::Id::unique("whats-new"))
        .frame(frame)
        .show(ctx, |ui| {
            let mut closed = false;
            ui.set_width(width);
            // The gaps between the bars and the list are set here; the notes
            // keep the usual gap between them.
            let note_gap = ui.spacing().item_spacing.y;
            ui.spacing_mut().item_spacing.y = 0.0;
            row_of(ui, BAR_HEIGHT, |ui| {
                let title = t!("whats-new-title", version = update::VERSION);
                ui.label(title_txt(&title, size::LARGE, None));
                far(ui, |ui| {
                    let tip = hint(&t!("whats-new-close"), "Escape");
                    let size = Vec2::splat(CLOSE_BUTTON);
                    closed |= icon_button_ex(ui, Icon::Close, &tip, size, CLOSE_CROSS, None, None)
                        .clicked();
                });
            });
            ui.add_space(space::MEDIUM);
            // As tall as set from the start: the sheet first opens with its
            // top at the middle of the window, where little room is left.
            egui::ScrollArea::vertical()
                .auto_shrink([false, true])
                .max_height(list_height)
                .min_scrolled_height(list_height)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = note_gap;
                    // The notes keep clear of the scroll bar's handle.
                    ui.set_max_width(ui.available_width() - super::list::handle_room(ui));
                    // The notes are in English: they read left to right in
                    // either language.
                    ui.with_layout(Layout::top_down(Align::Min), |ui| {
                        for note in notes {
                            note_row(ui, note);
                        }
                    });
                });
            ui.add_space(space::MEDIUM);
            row_of(ui, BAR_HEIGHT, |ui| {
                far(ui, |ui| {
                    closed |= primary_button(ui, &t!("whats-new-close")).clicked();
                });
            });
            (ui.min_rect(), closed)
        })
}

/// One note: a dot, then the note on as many lines as it needs.
fn note_row(ui: &mut Ui, note: &str) {
    let color = ui.visuals().text_color();
    // The dot's place and the gap after it, so the row keeps to its width.
    let text_w = (ui.available_width() - DOT_PLACE - ui.spacing().item_spacing.x).max(0.0);
    ui.horizontal_top(|ui| {
        let line_h = size::BODY * 1.45;
        let (dot, _) = ui.allocate_exact_size(Vec2::new(DOT_PLACE, line_h), Sense::hover());
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The sheet in a window `size` big, with the app's fonts and look,
    /// once it has settled.
    fn shown(size: Vec2) -> egui::ModalResponse<(Rect, bool)> {
        let ctx = egui::Context::default();
        add_fonts(&ctx);
        style(&ctx, crate::config::DarkPanels::Lighter);
        let notes = update::notes(update::VERSION);
        assert!(!notes.is_empty(), "no notes for {}", update::VERSION);
        let mut last = None;
        // The fonts apply from the next frame, and the sheet takes its place
        // from its size in the frame before.
        for _ in 0..4 {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| last = Some(sheet(ui.ctx(), &notes)));
            output.textures_delta.clear();
        }
        last.expect("a frame ran")
    }

    #[test]
    fn the_sheet_has_the_same_room_on_every_side() {
        // A maximized window at 125%, the docs' window, and a small one.
        for size in [
            Vec2::new(1536.0, 794.0),
            Vec2::new(1280.0, 800.0),
            Vec2::new(640.0, 480.0),
        ] {
            let response = shown(size);
            let (outer, (inner, _)) = (response.response.rect, response.inner);
            let start = inner.min - outer.min;
            let end = outer.max - inner.max;
            let at = format!("{size:?}: {inner:?} in {outer:?}");
            assert!((start.x - end.x).abs() < 0.5, "sides differ, {at}");
            assert!((start.y - end.y).abs() < 0.5, "top and bottom differ, {at}");
            assert!((start.x - start.y).abs() < 0.5, "side and top differ, {at}");
            assert!(inner.width() <= SHEET_WIDTH + 0.5, "too wide, {at}");
            assert!(outer.top() >= WINDOW_ROOM - 0.5, "near the top, {at}");
            assert!(
                outer.bottom() <= size.y - WINDOW_ROOM + 0.5,
                "near the bottom, {at}"
            );
        }
    }
}
