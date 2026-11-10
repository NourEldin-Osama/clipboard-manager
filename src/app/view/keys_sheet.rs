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
        far(ui, |ui| {
            // A far-end row lays the caps out backwards in a left-to-right
            // window; undo that.
            let keys = if i18n::rtl() {
                keys.to_string()
            } else {
                keys.split('+').rev().collect::<Vec<_>>().join("+")
            };
            shortcut::keycaps(ui, &keys, false);
        });
    });
    ui.add_space(space::TINY);
}
