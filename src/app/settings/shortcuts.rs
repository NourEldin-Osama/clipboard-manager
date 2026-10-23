//! Settings, Shortcuts: global and in-app keys, with the recorder.

use super::*;

/// Picks one shortcut out of the settings.
pub(super) type KeyField = fn(&mut Config) -> &mut String;

impl App {
    pub(super) fn shortcuts_page(&mut self, ui: &mut Ui, d: &mut Config, apply_keys: &mut bool) {
        let def = Config::default();
        section(ui, "Anywhere in Windows");
        let globals: [(&'static str, &str, &str, KeyField); 2] = [
            (
                "show",
                "Show or hide this window",
                "Opens the window over whatever you're doing; press again to hide it.",
                |c| &mut c.show_hotkey,
            ),
            (
                "ignore",
                "Don't record the next copy",
                "Press before copying something private: the next copy is skipped.",
                |c| &mut c.ignore_next_hotkey,
            ),
        ];
        for (id, label, help, field) in globals {
            let default = field(&mut def.clone()).clone();
            let value = field(d);
            let invalid = !value.trim().is_empty() && HotKey::from_str(value.trim()).is_err();
            let mut result = shortcut::Recorded::Nothing;
            let reset = setting(ui, label, help, *value != default, |ui| {
                result = shortcut::recorder(
                    ui,
                    value,
                    self.recording == Some(id),
                    true,
                    &mut self.record_problem,
                );
                if invalid {
                    chip(ui, "⚠ Not usable", RED);
                } else if self.hotkey_conflicts.iter().any(|k| k == value.trim()) {
                    chip(ui, "Used by another app", RED);
                }
            });
            if reset {
                *value = default;
                *apply_keys = true;
            }
            match result {
                shortcut::Recorded::Started => {
                    // Off while recording, so pressing the current keys doesn't hide the window.
                    self.hotkeys = None;
                    self.hotkeys_paused = true;
                    self.recording = Some(id);
                }
                shortcut::Recorded::Changed | shortcut::Recorded::Cancelled => {
                    self.recording = None;
                    *apply_keys = true;
                }
                shortcut::Recorded::Nothing => {}
            }
        }
        if let Some(e) = &self.hotkey_error {
            note(ui, &format!("⚠ {e}"));
        }

        section(ui, "Inside the app");
        let locals: [(&'static str, &str, KeyField); 5] = [
            ("copy", "Copy the selected item and hide the window", |c| {
                &mut c.keys.copy
            }),
            ("pin", "Pin or unpin", |c| &mut c.keys.pin),
            ("delete", "Delete", |c| &mut c.keys.delete),
            ("tag", "Add a tag", |c| &mut c.keys.tag),
            ("view", "Switch between Grid, List and Detail", |c| {
                &mut c.keys.view
            }),
        ];
        let current: Vec<(&str, String)> = locals
            .iter()
            .map(|(id, _, f)| (*id, f(&mut d.clone()).clone()))
            .collect();
        for (id, label, field) in locals {
            let default = field(&mut def.clone()).clone();
            let value = field(d);
            let clash = !value.is_empty()
                && current
                    .iter()
                    .any(|(other, v)| *other != id && v.eq_ignore_ascii_case(value));
            let mut result = shortcut::Recorded::Nothing;
            let reset = setting(ui, label, "", *value != default, |ui| {
                result = shortcut::recorder(
                    ui,
                    value,
                    self.recording == Some(id),
                    false,
                    &mut self.record_problem,
                );
                if clash {
                    chip(ui, "Used twice", AMBER);
                }
            });
            if reset {
                *value = default;
            }
            match result {
                shortcut::Recorded::Started => self.recording = Some(id),
                shortcut::Recorded::Changed | shortcut::Recorded::Cancelled => {
                    self.recording = None
                }
                shortcut::Recorded::Nothing => {}
            }
        }
        for (label, keys) in [
            ("Move through the list", "Up+Down"),
            ("Move between cards in the grid", "Left+Right"),
            ("Close details, clear the search, or hide", "Escape"),
        ] {
            plain_setting(ui, label, "Built in; can't be changed.", |ui| {
                shortcut::fixed(ui, keys)
            });
        }
        note(
            ui,
            "Click a shortcut, then press the new keys. Backspace turns it off, Esc cancels.",
        );
    }
}
