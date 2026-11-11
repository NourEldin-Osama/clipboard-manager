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
        // Sent from the main window's frame: eframe drops what the popup's
        // own frame sends while the main window is hidden or minimised.
        if showing {
            ctx.send_viewport_cmd_to(popup_id(), egui::ViewportCommand::Focus);
        }
        if std::mem::take(&mut self.popup_moving) {
            ctx.send_viewport_cmd_to(popup_id(), egui::ViewportCommand::StartDrag);
        }
    }

    /// The bar at the top: Settings and the full window at the start, a
    /// handle in the middle, Close at the end. Dragging the handle, or any
    /// empty part of the bar, moves the popup until it closes; it opens at
    /// the mouse pointer again.
    fn popup_bar(&mut self, ui: &mut Ui) {
        let bar = Rect::from_min_size(
            ui.available_rect_before_wrap().min,
            Vec2::new(ui.available_width(), 30.0),
        );
        // Made before the buttons, so they stay on top of it.
        let drag = ui
            // Not in the Tab order: the keyboard has nothing to do there.
            .interact(bar, ui.scope_id().with("move"), Sense::DRAG)
            .on_hover_cursor(egui::CursorIcon::Grab);
        if drag.drag_started() {
            self.popup_moving = true;
        }
        let handle = Rect::from_center_size(bar.center(), Vec2::new(36.0, 4.0));
        let weak = ui.visuals().weak_text_color();
        ui.painter().rect_filled(handle, 2.0, weak);
        row(ui, |ui| {
            let small = Vec2::splat(30.0);
            if icon_button_ex(
                ui,
                Icon::Settings,
                &t!("top-settings"),
                small,
                15.0,
                None,
                None,
            )
            .clicked()
            {
                self.open_settings(SettingsPage::General);
            }
            let tip = hint(&t!("popup-open-window"), &self.cfg.window_hotkey);
            if icon_button_ex(ui, Icon::Open, &tip, small, 15.0, None, None).clicked() {
                self.win.show_full();
            }
            far(ui, |ui| {
                let tip = hint(&t!("popup-close"), "Escape");
                if icon_button_ex(ui, Icon::Close, &tip, small, 15.0, None, None).clicked() {
                    self.win.hide();
                }
            });
        });
    }

    /// The quiet line at the bottom: each key, then what it does. Pasting,
    /// the numbers and F1 for the rest, so it stays one line; keys turned
    /// off in Settings are left out.
    fn key_hints(&self, ui: &mut Ui) {
        let k = &self.cfg.keys;
        let hints: Vec<(String, String)> = [
            (k.paste.clone(), t!("popup-key-paste")),
            (t!("quick-paste-keys"), t!("popup-key-quick")),
            ("F1".to_string(), t!("popup-key-all")),
        ]
        .into_iter()
        .filter(|(keys, _)| !keys.trim().is_empty())
        .collect();
        // While searching, how many results there are, as the window says
        // beside its filters.
        if !self.query.trim().is_empty() {
            let text = t!("top-results", count = self.results.len());
            let weak = ui.visuals().weak_text_color();
            row(ui, |ui| lbl(ui, &text, size::SMALL, weak));
        }
        paint_key_hints(ui, &hints);
    }

    /// The bar, the search field and the list. The keys work as in the full
    /// window.
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
        self.keyboard_focus = keyboard_moved_focus(&ctx, self.keyboard_focus);
        self.keyboard(&ctx, true);
        let fill = ui.visuals().panel_fill;
        egui::Panel::top("popup-bar")
            .show_separator_line(false)
            .frame(
                Frame::new()
                    .inner_margin(Margin {
                        left: 6,
                        right: 6,
                        top: 4,
                        bottom: 0,
                    })
                    .fill(fill),
            )
            .show(ui, |ui| self.popup_bar(ui));
        egui::Panel::top("popup-top")
            .show_separator_line(false)
            .frame(
                Frame::new()
                    .inner_margin(Margin {
                        left: 10,
                        right: 10,
                        top: 4,
                        bottom: 6,
                    })
                    .fill(fill),
            )
            .show(ui, |ui| {
                let width = ui.available_width();
                self.search_field(ui, width);
            });
        egui::Panel::bottom("popup-keys")
            .show_separator_line(false)
            .frame(
                Frame::new()
                    .inner_margin(Margin {
                        left: 10,
                        right: 10,
                        top: 2,
                        bottom: 8,
                    })
                    .fill(fill),
            )
            .show(ui, |ui| self.key_hints(ui));
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
                        let red = text_shade(RED, ui.visuals().dark_mode);
                        icons::glyph(ui, Icon::Warning, 13.0, red);
                        let room = ui.available_width();
                        let r = cut_line(ui, &error, room, size::SMALL, red, Sense::click());
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
        self.toasts(&ctx, true);
        self.keys_sheet(&ctx, true);
        // Settings opened here, by a link in the empty list, need the full window.
        if !had_settings && self.draft.is_some() {
            self.win.show_full();
        }
        // Without a title bar, a thin edge sets the popup apart from what is
        // behind it: the edge menus have. (The theme leaves out the outline
        // of things you can't click, which this used before and was empty.)
        let edge = ui.visuals().window_stroke.color;
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
        if self.keyboard_focus {
            focus_outline(&ctx);
        }
        bidi::read_in_reading_order(&ctx);
    }
}

/// Where pairs `widths` wide go in lines `room` wide: each pair's line and
/// where it starts in it. The pairs of a line spread across it with equal
/// gaps; a lone pair is centred. In a narrow line the gaps shrink first,
/// down to `least`, before a pair starts the next line.
fn spread(widths: &[f32], room: f32, least: f32) -> Vec<(usize, f32)> {
    let mut lines: Vec<Vec<f32>> = Vec::new();
    for &width in widths {
        match lines.last_mut() {
            Some(line) if line.iter().map(|w| w + least).sum::<f32>() + width <= room => {
                line.push(width);
            }
            _ => lines.push(vec![width]),
        }
    }
    let mut places = Vec::new();
    for (row, line) in lines.iter().enumerate() {
        let total: f32 = line.iter().sum();
        let (mut x, gap) = match line.len() {
            1 => ((room - total) / 2.0, 0.0),
            n => (0.0, (room - total) / (n - 1) as f32),
        };
        for width in line {
            places.push((row, x));
            x += width + gap;
        }
    }
    places
}

/// Where a galley's first line stands, from its top.
fn baseline(galley: &egui::Galley) -> f32 {
    galley
        .rows
        .first()
        .and_then(|row| row.row.glyphs.first().map(|glyph| row.pos.y + glyph.pos.y))
        .unwrap_or(galley.size().y)
}

/// Pairs of keys and what they do, spread across a line with equal gaps:
/// the keys as key caps, as Settings shows them, and the words on the line
/// the caps' names stand on. A pair that doesn't fit on a line, even with
/// the gaps at their least, starts the next one.
fn paint_key_hints(ui: &mut Ui, hints: &[(String, String)]) {
    let (strong, weak) = (ui.visuals().text_color(), ui.visuals().weak_text_color());
    let dark = ui.visuals().dark_mode;
    let room = ui.available_width().max(0.0);
    let font = FontId::proportional(size::SMALL);
    // Between the caps of one shortcut.
    let cap_gap = 3.0;
    let pairs: Vec<_> = hints
        .iter()
        .map(|(keys, what)| {
            let mut caps = Vec::new();
            for part in keys.split('+').map(str::trim).filter(|k| !k.is_empty()) {
                // A range such as "1 to 9" is two keys with the word between.
                let words: Vec<&str> = part.split_whitespace().collect();
                if words.len() >= 3 {
                    let between = words[1..words.len() - 1].join(" ");
                    caps.push(Piece::Cap(shortcut::keycap_label(ui, words[0], strong)));
                    caps.push(Piece::Word(one_line(
                        ui,
                        &between,
                        &[],
                        font.clone(),
                        weak,
                        room,
                    )));
                    let last = words[words.len() - 1];
                    caps.push(Piece::Cap(shortcut::keycap_label(ui, last, strong)));
                } else {
                    caps.push(Piece::Cap(shortcut::keycap_label(ui, part, strong)));
                }
            }
            // What the keys do in the text color: the hints are the popup's
            // way to its keys, and grey read as switched off.
            (caps, one_line(ui, what, &[], font.clone(), strong, room))
        })
        .collect();
    let piece_width = |piece: &Piece| match piece {
        Piece::Cap(g) => shortcut::keycap_size(g, HINT_CAP).x,
        Piece::Word(g) => g.size().x,
    };
    let caps_width = |caps: &[Piece]| {
        caps.iter().map(piece_width).sum::<f32>() + cap_gap * caps.len().saturating_sub(1) as f32
    };
    // Between the keys and their words, and the least between pairs.
    let (gap, least) = (6.0, space::MEDIUM);
    let widths: Vec<f32> = pairs
        .iter()
        .map(|(caps, what)| caps_width(caps) + gap + what.size().x)
        .collect();
    let places = spread(&widths, room, least);
    let lines = places.last().map_or(0, |(row, _)| row + 1);
    let line_h = HINT_CAP + 6.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(room, line_h * lines as f32), Sense::hover());
    let p = ui.painter();
    for ((caps, what), (row, start)) in pairs.iter().zip(places) {
        let y = rect.top() + line_h * (row as f32 + 0.5);
        let mut x = rect.left() + start;
        // The keys as one group, mirrored for Arabic as a whole, so
        // they keep the order Settings shows them in.
        let group = flip(
            rect,
            Rect::from_min_size(
                Pos2::new(x, y - HINT_CAP / 2.0),
                Vec2::new(caps_width(caps), HINT_CAP),
            ),
        );
        // The line the caps' names stand on, in the middle of the caps;
        // the words stand on it too.
        let base = caps
            .iter()
            .find_map(|piece| match piece {
                Piece::Cap(g) => Some(y - g.size().y / 2.0 + baseline(g)),
                Piece::Word(_) => None,
            })
            .unwrap_or(y);
        let mut cap_x = group.left();
        for piece in caps {
            match piece {
                Piece::Cap(g) => {
                    let size = shortcut::keycap_size(g, HINT_CAP);
                    let cap = Rect::from_min_size(Pos2::new(cap_x, group.top()), size);
                    shortcut::paint_keycap(p, cap, g.clone(), strong, dark);
                }
                Piece::Word(g) => {
                    p.galley(Pos2::new(cap_x, base - baseline(g)), g.clone(), weak);
                }
            }
            cap_x += piece_width(piece) + cap_gap;
        }
        x += group.width() + gap;
        let size = what.size();
        let at = flip(
            rect,
            Rect::from_min_size(Pos2::new(x, base - baseline(what)), size),
        );
        p.galley(at.min, what.clone(), strong);
    }
}

/// A part of a shortcut in the popup's hints: a key, or a word between two
/// keys, such as "to" in "1 to 9".
enum Piece {
    Cap(Arc<egui::Galley>),
    Word(Arc<egui::Galley>),
}

/// How tall the popup's key caps are: a little smaller than Settings', so
/// two lines of them leave room for the list.
const HINT_CAP: f32 = 22.0;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hints_spread_with_equal_gaps_and_shrink_them_before_wrapping() {
        // Room to spare: the first at the start, the last at the end.
        assert_eq!(
            spread(&[100.0, 120.0, 110.0], 420.0, 12.0),
            [(0, 0.0), (0, 145.0), (0, 310.0)]
        );
        // Just room for the least gaps: still one line.
        assert_eq!(
            spread(&[100.0, 120.0, 110.0], 354.0, 12.0),
            [(0, 0.0), (0, 112.0), (0, 244.0)]
        );
        // Less: the last one goes on a line of its own, centred.
        assert_eq!(
            spread(&[100.0, 120.0, 110.0], 353.0, 12.0),
            [(0, 0.0), (0, 233.0), (1, 121.5)]
        );
    }

    /// The popup's hints in `language`, at the popup's width: every text
    /// on them, the caps' names and the words, stands on one line, and the
    /// hints reach from the line's start to its end.
    fn check_hints_on_one_line(language: crate::config::Language) {
        let ctx = egui::Context::default();
        add_fonts(&ctx);
        style(&ctx, crate::config::DarkPanels::Lighter);
        // The fonts apply from the next frame.
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .textures_delta
            .clear();
        let text = |key: &str| crate::i18n::text_in(language, key, None);
        let hints = [
            ("Enter".to_string(), text("popup-key-paste")),
            (text("quick-paste-keys"), text("popup-key-quick")),
            ("F1".to_string(), text("popup-key-all")),
        ];
        // The popup's width less its hints' margins.
        let line = Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(POPUP_SIZE.x - 20.0, 60.0));
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.scope_builder(egui::UiBuilder::new().max_rect(line), |ui| {
                paint_key_hints(ui, &hints);
            });
        });
        output.textures_delta.clear();
        let (mut baselines, mut start, mut end) = (Vec::new(), f32::INFINITY, f32::NEG_INFINITY);
        for clipped in &output.shapes {
            match &clipped.shape {
                egui::Shape::Text(text) => {
                    baselines.push(text.pos.y + baseline(&text.galley));
                    end = end.max(text.pos.x + text.galley.rect.right());
                }
                egui::Shape::Rect(shape) => start = start.min(shape.rect.left()),
                _ => {}
            }
        }
        // Enter, Ctrl, 1, to, 9, F1 and the three words.
        assert_eq!(baselines.len(), 9, "{language:?}");
        for at in &baselines {
            assert!(
                (at - baselines[0]).abs() <= 0.5,
                "{language:?}: {baselines:?}"
            );
        }
        assert!((start - line.left()).abs() <= 0.5, "{language:?}: {start}");
        assert!((end - line.right()).abs() <= 1.0, "{language:?}: {end}");
    }

    #[test]
    fn the_hints_fit_on_one_line_in_english_and_arabic() {
        check_hints_on_one_line(crate::config::Language::English);
        check_hints_on_one_line(crate::config::Language::Arabic);
    }
}
