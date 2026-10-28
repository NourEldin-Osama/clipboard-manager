//! The quick popup: a small window at the mouse pointer with the search field
//! and the list. It is a window of its own, made each time it opens and
//! closed with it; the full window stays where it is.

use super::*;

/// The popup's size, in points.
const POPUP_SIZE: Vec2 = Vec2::new(440.0, 520.0);

fn popup_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("popup")
}

/// Whether the popup's window exists (it may be hidden).
fn popup_made(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.raw.viewports.contains_key(&popup_id()))
}

fn popup_builder() -> egui::ViewportBuilder {
    egui::ViewportBuilder::default()
        .with_title(t!("app-title"))
        .with_decorations(false)
        .with_resizable(false)
        .with_taskbar(false)
        .with_window_level(egui::WindowLevel::AlwaysOnTop)
        .with_inner_size(POPUP_SIZE)
}

impl App {
    /// The main window makes the popup's window while drawing, and draws
    /// nothing while minimised: one drawn frame is asked for so the popup
    /// can open. Runs from `logic`, which runs in every frame.
    pub(super) fn popup_logic(&mut self, ctx: &egui::Context) {
        if self.win.is_popup_open() && !popup_made(ctx) {
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaintWhileHidden);
            ctx.request_repaint();
        }
    }

    /// Draws the popup's window while it is open. A frame without it closes
    /// the window.
    pub(super) fn popup_window(&mut self, ctx: &egui::Context) {
        if !self.win.is_popup_open() {
            return;
        }
        if !popup_made(ctx) {
            // eframe 0.37 makes the window of a "deferred" viewport itself,
            // after the frame. An "immediate" one is made during the frame,
            // which needs a context eframe lacks while the main window is
            // hidden or minimised (it draws such a window from
            // `check_redraw_requests` in its run.rs), and egui then panics.
            // So the window is made empty and hidden first, and drawn
            // immediately from the next frame on.
            ctx.show_viewport_deferred(popup_id(), popup_builder().with_visible(false), |_, _| {});
            ctx.request_repaint();
            return;
        }
        let placing = self.win.popup_placing.swap(false, Ordering::SeqCst);
        if placing {
            // Each time it opens, the popup starts from the newest copies.
            self.query.clear();
            self.query_vec = None;
            self.filter = Filter::All;
            self.dirty = true;
            self.popup_at = popup_corner(ctx);
            ctx.request_repaint();
        }
        // Shown from its second frame, once there is something drawn in it.
        let mut builder = popup_builder().with_visible(!placing);
        if let Some(at) = self.popup_at {
            builder = builder.with_position(at);
        }
        let showing = self.popup_hidden_last_frame && !placing;
        self.popup_hidden_last_frame = placing;
        ctx.show_viewport_immediate(popup_id(), builder, |ui, _| self.popup_ui(ui));
        if showing {
            // Sent from the main window's frame: eframe drops what the popup's
            // own frame sends while the main window is hidden or minimised.
            ctx.send_viewport_cmd_to(popup_id(), egui::ViewportCommand::Focus);
        }
    }

    /// The search field, a button for the full window, and the list. The
    /// keys work as in the full window.
    fn popup_ui(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(false));
        // Like a menu, the popup closes when the user clicks elsewhere.
        let left = self.win.popup_focused.swap(focused, Ordering::SeqCst) && !focused;
        if left || ctx.input(|i| i.viewport().close_requested()) {
            self.win.hide();
            self.left_for_another_program();
            return;
        }
        let had_settings = self.draft.is_some();
        self.keyboard(&ctx, true);
        let fill = ui.visuals().panel_fill;
        egui::Panel::top("popup-top")
            .show_separator_line(false)
            .frame(
                Frame::new()
                    .inner_margin(Margin {
                        left: 10,
                        right: 8,
                        top: 10,
                        bottom: 6,
                    })
                    .fill(fill),
            )
            .show(ui, |ui| {
                row(ui, |ui| {
                    let width = (ui.available_width() - 44.0).max(160.0);
                    self.search_field(ui, width);
                    far(ui, |ui| {
                        let tip = hint(&t!("popup-open-window"), &self.cfg.window_hotkey);
                        if icons::icon_button(ui, Icon::Open, &tip).clicked() {
                            self.win.show_full();
                        }
                    });
                });
            });
        if let Some((error, _)) = self.error.clone() {
            egui::Panel::bottom("popup-error")
                .show_separator_line(false)
                .frame(
                    Frame::new()
                        .inner_margin(Margin::symmetric(12, 6))
                        .fill(fill),
                )
                .show(ui, |ui| {
                    row(ui, |ui| {
                        icons::glyph(ui, Icon::Warning, 13.0, RED);
                        let room = ui.available_width();
                        let r = cut_line(ui, &error, room, 12.0, RED, Sense::click());
                        if tip(r, &t!("status-dismiss-error")).clicked() {
                            self.error = None;
                        }
                    });
                });
        }
        egui::CentralPanel::default()
            .frame(
                Frame::new()
                    .inner_margin(Margin {
                        left: 6,
                        right: 6,
                        top: 2,
                        bottom: 6,
                    })
                    .fill(fill),
            )
            .show(ui, |ui| self.list(ui, true));
        self.toasts(&ctx);
        // Settings opened here, by a link in the empty list, need the full window.
        if !had_settings && self.draft.is_some() {
            self.win.show_full();
        }
        // Without a title bar, a thin edge sets the popup apart from what is
        // behind it.
        let edge = ui.visuals().widgets.noninteractive.bg_stroke.color;
        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::unique("popup-edge"),
        ))
        .rect_stroke(
            ctx.content_rect(),
            0.0,
            Stroke::new(1.0, edge),
            egui::StrokeKind::Inside,
        );
    }
}

/// Where the popup goes: its corner at the mouse pointer, inside the work
/// area of the pointer's screen, in points. `None` where the pointer can't
/// be read.
fn popup_corner(ctx: &egui::Context) -> Option<Pos2> {
    let pointer = crate::screen::pointer()?;
    let zoom = ctx.zoom_factor();
    let size = POPUP_SIZE * pointer.scale * zoom;
    let corner = crate::screen::popup_corner(pointer.at, pointer.work_area, size, i18n::rtl());
    // The popup's window turns points into pixels with the scale of the
    // screen it is on before it moves: where it is now, or for a new window
    // most likely the pointer's (it may differ with screens of mixed scales).
    let scale = ctx
        .input(|i| {
            i.raw
                .viewports
                .get(&popup_id())
                .and_then(|v| v.native_pixels_per_point)
        })
        .unwrap_or(pointer.scale);
    Some((corner.to_vec2() / (scale * zoom)).to_pos2())
}
