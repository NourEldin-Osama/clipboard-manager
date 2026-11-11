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
        list.extend([
            (t!("settings-shortcut-menu"), "Shift+F10".into()),
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
        let groups = self.key_groups(in_popup);
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
            egui::ScrollArea::vertical()
                .max_height(tallest)
                .min_scrolled_height(tallest)
                .show(ui, |ui| {
                    // Room for the scroll bar, which floats over the content.
                    ui.set_max_width(ui.available_width() - 12.0);
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
        if close || response.should_close() {
            self.keys_sheet = None;
        }
    }
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
