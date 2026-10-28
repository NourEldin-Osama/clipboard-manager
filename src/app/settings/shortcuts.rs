//! Settings, Shortcuts: global and in-app keys, with the recorder.

use super::*;

/// Picks one shortcut out of the settings.
pub(super) type KeyField = fn(&mut Config) -> &mut String;

impl App {
    pub(super) fn shortcuts_page(&mut self, ui: &mut Ui, d: &mut Config, apply_keys: &mut bool) {
        let def = Config::default();
        section(ui, &t!("settings-shortcuts-global"));
        let globals: [(ShortcutField, String, String, KeyField); 2] = [
            (
                ShortcutField::Show,
                t!("settings-shortcut-show"),
                t!("settings-shortcut-show-help"),
                |c| &mut c.show_hotkey,
            ),
            (
                ShortcutField::IgnoreNext,
                t!("settings-shortcut-ignore"),
                t!("settings-shortcut-ignore-help"),
                |c| &mut c.ignore_next_hotkey,
            ),
        ];
        for (id, label, help, field) in globals {
            let default = field(&mut def.clone()).clone();
            let value = field(d);
            let invalid = !value.trim().is_empty() && HotKey::from_str(value.trim()).is_err();
            let mut result = shortcut::Recorded::Nothing;
            let reset = setting(ui, &label, &help, *value != default, |ui| {
                result = shortcut::recorder(
                    ui,
                    value,
                    self.recording == Some(id),
                    true,
                    &mut self.record_problem,
                );
                if invalid {
                    icons::chip(ui, Icon::Warning, &t!("settings-shortcut-not-usable"), RED);
                } else if self.hotkey_conflicts.iter().any(|k| k == value.trim()) {
                    chip(ui, t!("settings-shortcut-taken"), RED);
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

        section(ui, &t!("settings-shortcuts-local"));
        let locals: [(ShortcutField, String, String, KeyField); 7] = [
            (
                ShortcutField::Paste,
                t!("settings-shortcut-paste"),
                t!("settings-shortcut-paste-help"),
                |c| &mut c.keys.paste,
            ),
            (
                ShortcutField::PastePlain,
                t!("settings-shortcut-paste-plain"),
                String::new(),
                |c| &mut c.keys.paste_plain,
            ),
            (
                ShortcutField::CopyOnly,
                t!("settings-shortcut-copy-only"),
                String::new(),
                |c| &mut c.keys.copy_only,
            ),
            (
                ShortcutField::Pin,
                t!("settings-shortcut-pin"),
                String::new(),
                |c| &mut c.keys.pin,
            ),
            (
                ShortcutField::Delete,
                t!("settings-shortcut-delete"),
                String::new(),
                |c| &mut c.keys.delete,
            ),
            (
                ShortcutField::Tag,
                t!("settings-shortcut-tag"),
                String::new(),
                |c| &mut c.keys.tag,
            ),
            (
                ShortcutField::View,
                t!("settings-shortcut-view"),
                String::new(),
                |c| &mut c.keys.view,
            ),
        ];
        let current: Vec<(ShortcutField, String)> = locals
            .iter()
            .map(|(id, _, _, f)| (*id, f(&mut d.clone()).clone()))
            .collect();
        for (id, label, help, field) in locals {
            let default = field(&mut def.clone()).clone();
            let value = field(d);
            let clash = !value.is_empty()
                && current
                    .iter()
                    .any(|(other, v)| *other != id && v.eq_ignore_ascii_case(value));
            let mut result = shortcut::Recorded::Nothing;
            let reset = setting(ui, &label, &help, *value != default, |ui| {
                result = shortcut::recorder(
                    ui,
                    value,
                    self.recording == Some(id),
                    false,
                    &mut self.record_problem,
                );
                if clash {
                    chip(ui, t!("settings-shortcut-used-twice"), AMBER);
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
            (t!("settings-shortcut-move"), "Up+Down"),
            (t!("settings-shortcut-move-grid"), "Left+Right"),
            (t!("settings-shortcut-escape"), "Escape"),
        ] {
            plain_setting(ui, &label, &t!("settings-shortcut-built-in"), |ui| {
                shortcut::fixed(ui, keys)
            });
        }
        note(ui, &t!("settings-shortcuts-note"));
    }
}
