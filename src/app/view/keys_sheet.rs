//! The sheet of every key: F1 opens it over the full window or the popup;
//! F1, Esc, its Close button or a click beside it closes it.

use super::*;

/// What a key does, then its keys.
type KeyRow = (String, String);

impl App {
    /// Every key that works in this window, by where it works, leaving out
    /// the ones turned off in Settings.
    fn key_groups(&self, in_popup: bool) -> Vec<(String, Vec<KeyRow>)> {
        let c = &self.cfg;
        let k = &c.keys;
        let mut list: Vec<KeyRow> = vec![(t!("settings-shortcut-move"), "Up+Down".into())];
        // The popup has neither the grid, nor tags, nor views.
        if !in_popup {
            list.push((t!("settings-shortcut-move-grid"), "Left+Right".into()));
        }
        list.extend([
            (t!("settings-shortcut-paste"), k.paste.clone()),
            (t!("settings-shortcut-paste-plain"), k.paste_plain.clone()),
            (t!("settings-shortcut-copy-only"), k.copy_only.clone()),
            (t!("settings-shortcut-quick-paste"), t!("quick-paste-keys")),
            (t!("settings-shortcut-pin"), k.pin.clone()),
        ]);
        if !in_popup {
            list.push((t!("settings-shortcut-tag"), k.tag.clone()));
            list.push((t!("settings-shortcut-details"), k.details.clone()));
        }
        list.push((t!("settings-shortcut-delete"), k.delete.clone()));
        if !in_popup {
            list.push((t!("settings-shortcut-view"), k.view.clone()));
        }
        list.push((t!("settings-shortcut-menu"), "Shift+F10".into()));
        // Full screen takes the full window, which the popup isn't.
        if !in_popup {
            list.push((t!("settings-shortcut-full-screen"), "F11".into()));
        }
        list.extend([
            (t!("settings-shortcut-escape"), "Escape".into()),
            (t!("settings-shortcut-keys"), "F1".into()),
        ]);
        let everywhere = vec![
            (t!("settings-shortcut-popup"), c.popup_hotkey.clone()),
            (t!("settings-shortcut-window"), c.window_hotkey.clone()),
            (t!("settings-shortcut-ignore"), c.ignore_next_hotkey.clone()),
        ];
        [
            (t!("settings-shortcuts-local"), list),
            (t!("settings-shortcuts-global"), everywhere),
        ]
        .into_iter()
        .map(|(title, rows)| {
            let rows = rows.into_iter().filter(|(_, keys)| !keys.trim().is_empty());
            (title, rows.collect())
        })
        .collect()
    }

    /// Draws the sheet while it is open over this window.
    pub(super) fn keys_sheet(&mut self, ctx: &egui::Context, in_popup: bool) {
        if self.keys_sheet != Some(in_popup) {
            return;
        }
        if sheet(ctx, &self.key_groups(in_popup)) {
            self.keys_sheet = None;
        }
    }
}

/// The sheet of the keys in `groups` over the window; whether it was
/// closed.
fn sheet(ctx: &egui::Context, groups: &[(String, Vec<KeyRow>)]) -> bool {
    let room = ctx.content_rect().size();
    let mut close = false;
    let response = egui::Modal::new(egui::Id::unique("keys-sheet")).show(ctx, |ui| {
        ui.set_width((room.x - 48.0).min(560.0));
        row(ui, |ui| {
            ui.label(title_txt(&t!("keys-sheet-title"), size::LARGE, None));
            far(ui, |ui| {
                let tip = hint(&t!("keys-sheet-close"), "F1");
                let size = Vec2::splat(28.0);
                close = icon_button_ex(ui, Icon::Close, &tip, size, 13.0, None, None).clicked();
            });
        });
        // As tall as the window allows: the sheet first opens with its
        // top at the middle of the window, where little room is left.
        let tallest = (room.y - 120.0).max(120.0);
        // As wide as the sheet, not as its rows, so that the scroll bar's
        // handle floats at the sheet's edge; the rows keep clear of it as
        // the list's rows do, on the same side in every language.
        egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .max_height(tallest)
            .min_scrolled_height(tallest)
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width() - super::list::handle_room(ui));
                let weak = ui.visuals().weak_text_color();
                for (title, rows) in groups.iter().filter(|(_, rows)| !rows.is_empty()) {
                    ui.add_space(space::MEDIUM);
                    row(ui, |ui| ui.label(section_title(title, weak)));
                    for (what, keys) in rows {
                        key_row(ui, what, keys);
                    }
                }
            });
    });
    close || response.should_close()
}

/// One key: what it does at the reading start, on as many lines as it
/// needs, and its keys at the far end.
fn key_row(ui: &mut Ui, what: &str, keys: &str) {
    let width = ui.available_width();
    let keys_w = 170.0_f32.min(width * 0.45);
    let text_w = (width - keys_w - 8.0).max(0.0);
    let color = ui.visuals().text_color();
    row(ui, |ui| {
        let start = if i18n::rtl() { Align::Max } else { Align::Min };
        ui.allocate_ui_with_layout(Vec2::new(text_w, 0.0), Layout::top_down(start), |ui| {
            ui.set_width(text_w);
            let font = FontId::proportional(size::CONTROL);
            if bidi::has_rtl(what) {
                bidi::paragraph(ui, what, font, color);
            } else {
                let text = RichText::new(what).font(font).color(color);
                ui.add(egui::Label::new(text).wrap());
            }
        });
        far(ui, |ui| shortcut::keycaps(ui, keys, false));
    });
    ui.add_space(space::TINY);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Draws one place that shows keys.
    type Draw<'a> = &'a dyn Fn(&mut Ui);

    /// What one frame of `draw` paints in a window 600 points wide, with
    /// the app's fonts.
    fn drawn(draw: Draw) -> Vec<egui::epaint::ClippedShape> {
        let ctx = egui::Context::default();
        add_fonts(&ctx);
        style(&ctx, crate::config::DarkPanels::Lighter);
        // The fonts apply from the next frame.
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .textures_delta
            .clear();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(600.0, 400.0))),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| draw(ui));
        output.textures_delta.clear();
        output.shapes
    }

    /// Ctrl+1 to 9 reads in order in every place that shows it: the
    /// popup's hints, this sheet, the empty details pane and Settings,
    /// Shortcuts.
    fn check_quick_paste_everywhere() {
        let keys = t!("quick-paste-keys");
        let hints = [(keys.clone(), t!("popup-key-quick"))];
        let what = t!("settings-shortcut-quick-paste");
        let pane_what = t!("details-key-quick");
        let places: [(&str, Draw); 4] = [
            ("popup", &|ui| {
                super::super::popup::paint_key_hints(ui, &hints)
            }),
            ("sheet", &|ui| key_row(ui, &what, &keys)),
            ("empty pane", &|ui| {
                super::super::details::key_hint_row(ui, &keys, &pane_what, Color32::GRAY)
            }),
            ("settings", &|ui| shortcut::fixed(ui, &keys)),
        ];
        for (place, draw) in places {
            if let Err(problem) = shortcut::quick_paste_order(&drawn(draw)) {
                panic!("{place}: {problem}");
            }
        }
    }

    /// The sheet in a window too short for all its rows, so that its list
    /// scrolls: nothing a row paints, its words or its key caps, comes
    /// within the scroll handle's width and 4 points of the list's edge,
    /// where the handle floats in every language; Arabic words as laid
    /// out, their letters drawn at most a point further.
    fn check_sheet_clear_of_the_handle() {
        let ctx = egui::Context::default();
        add_fonts(&ctx);
        icons::register(&ctx);
        style(&ctx, crate::config::DarkPanels::Lighter);
        // The scroll bar in its place at once.
        ctx.all_styles_mut(|style| style.animation_time = 0.0);
        // The fonts apply from the next frame.
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .textures_delta
            .clear();
        let row = |what: String, keys: &str| (what, keys.to_string());
        let groups = [
            (
                t!("settings-shortcuts-local"),
                vec![
                    row(t!("settings-shortcut-move"), "Up+Down"),
                    row(t!("settings-shortcut-move-grid"), "Left+Right"),
                    row(t!("settings-shortcut-paste"), "Enter"),
                    row(t!("settings-shortcut-paste-plain"), "Shift+Enter"),
                    row(t!("settings-shortcut-copy-only"), "Ctrl+C"),
                    (t!("settings-shortcut-quick-paste"), t!("quick-paste-keys")),
                    row(t!("settings-shortcut-pin"), "Ctrl+P"),
                    row(t!("settings-shortcut-tag"), "Ctrl+T"),
                    row(t!("settings-shortcut-details"), "Alt+Enter"),
                    row(t!("settings-shortcut-delete"), "Shift+Delete"),
                    row(t!("settings-shortcut-view"), "Ctrl+L"),
                    row(t!("settings-shortcut-menu"), "Shift+F10"),
                    row(t!("settings-shortcut-escape"), "Escape"),
                    row(t!("settings-shortcut-keys"), "F1"),
                ],
            ),
            (
                t!("settings-shortcuts-global"),
                vec![
                    row(t!("settings-shortcut-popup"), "Ctrl+Shift+V"),
                    row(t!("settings-shortcut-window"), "Ctrl+Shift+H"),
                    row(t!("settings-shortcut-ignore"), "Ctrl+Shift+I"),
                ],
            ),
        ];
        let input = || egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(600.0, 400.0))),
            ..Default::default()
        };
        // The bar shows from the frame after the one that finds the list
        // longer than its place.
        let mut shapes = Vec::new();
        for _ in 0..3 {
            let mut output = ctx.run_ui(input(), |ui| {
                sheet(ui.ctx(), &groups);
            });
            output.textures_delta.clear();
            shapes = output.shapes;
        }
        let rtl = i18n::rtl();
        // The list's own shapes are clipped to it, as its first group's
        // title is; the handle floats at its edge.
        let title = bidi::job(&groups[0].0, semibold(size::SMALL), Color32::GRAY).text;
        let list = shapes
            .iter()
            .find(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => text.galley.text() == title,
                _ => false,
            })
            .map(|clipped| clipped.clip_rect)
            .unwrap_or_else(|| panic!("rtl {rtl}: the list"));
        let handles: Vec<Rect> = shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(shape)
                    if shape.fill.a() > 0
                        && shape.rect.width() < 6.0
                        && shape.rect.height() > 20.0 =>
                {
                    Some(shape.rect)
                }
                _ => None,
            })
            .collect();
        assert_eq!(handles.len(), 1, "rtl {rtl}: {handles:?}");
        let handle = handles[0];
        let room = handle.width() + space::TINY;
        // Each part as laid out, a text as its line, and as drawn.
        let rows: Vec<(Rect, Rect)> = shapes
            .iter()
            .filter(|clipped| clipped.clip_rect == list)
            .map(|clipped| {
                let drawn = clipped.shape.visual_bounding_rect();
                match &clipped.shape {
                    egui::Shape::Text(text) => {
                        (text.galley.rect.translate(text.pos.to_vec2()), drawn)
                    }
                    _ => (drawn, drawn),
                }
            })
            .filter(|(_, drawn)| drawn.is_positive())
            .collect();
        assert!(rows.len() > 20, "rtl {rtl}: {rows:?}");
        let end = handle.right() - room;
        for (laid_out, drawn) in rows {
            assert!(
                laid_out.right() <= end + 0.5,
                "rtl {rtl}: {laid_out:?}, the handle {handle:?}"
            );
            // An Arabic letter at its line's reading start can draw a
            // little past the line, here up to 0.62 points.
            assert!(
                drawn.right() <= end + 1.0,
                "rtl {rtl}: {drawn:?}, the handle {handle:?}"
            );
        }
    }

    #[test]
    fn the_sheets_rows_keep_clear_of_the_scroll_handle() {
        check_sheet_clear_of_the_handle();
    }

    #[test]
    fn the_sheets_rows_keep_clear_of_the_scroll_handle_right_to_left() {
        crate::i18n::passes_in_arabic(
            module_path!(),
            "the_sheets_rows_keep_clear_of_the_scroll_handle_in_arabic",
        );
    }

    #[test]
    #[ignore = "run in a process of its own by the_sheets_rows_keep_clear_of_the_scroll_handle_right_to_left"]
    fn the_sheets_rows_keep_clear_of_the_scroll_handle_in_arabic() {
        if crate::i18n::arabic_run() {
            check_sheet_clear_of_the_handle();
        }
    }

    #[test]
    fn the_quick_paste_keys_read_in_order_everywhere() {
        check_quick_paste_everywhere();
    }

    #[test]
    fn the_quick_paste_keys_read_in_order_everywhere_right_to_left() {
        crate::i18n::passes_in_arabic(
            module_path!(),
            "the_quick_paste_keys_read_in_order_everywhere_in_arabic",
        );
    }

    #[test]
    #[ignore = "run in a process of its own by the_quick_paste_keys_read_in_order_everywhere_right_to_left"]
    fn the_quick_paste_keys_read_in_order_everywhere_in_arabic() {
        if crate::i18n::arabic_run() {
            check_quick_paste_everywhere();
        }
    }
}
