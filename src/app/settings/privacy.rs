//! Settings, Privacy: what is never kept, and where your data goes.

use super::*;

impl App {
    pub(super) fn privacy_page(&mut self, ui: &mut Ui, d: &mut Config) {
        let def = Config::default();
        section(ui, &t!("settings-never-kept"));
        setting_for(
            ui,
            &t!("settings-skip-secrets"),
            &t!("settings-skip-secrets-hint"),
            &t!("settings-skip-secrets-help"),
            &mut d.skip_secrets,
            def.skip_secrets,
            toggle,
        );
        setting_for(
            ui,
            &t!("settings-screen-notes"),
            &t!("settings-screen-notes-hint"),
            &t!("settings-screen-notes-help"),
            &mut d.screen_notes,
            def.screen_notes,
            toggle,
        );
        let mut change = false;
        plain_setting(
            ui,
            &t!("settings-skip-next"),
            &t!("settings-skip-next-hint"),
            "",
            |ui| {
                change = ui
                    .button(txt(&t!("settings-change-key"), size::BODY, None))
                    .clicked();
                shortcut::fixed(ui, &d.ignore_next_hotkey);
            },
        );
        if change {
            self.settings_page = SettingsPage::Shortcuts;
        }
        rules_section(ui, d);
        self.where_data_goes(ui, d);
    }

    /// Where each kind of data stays or goes, as the settings are now.
    fn where_data_goes(&self, ui: &mut Ui, d: &Config) {
        section(ui, &t!("settings-data-goes"));
        let dark = ui.visuals().dark_mode;
        let (weak, warn) = (ui.visuals().weak_text_color(), text_shade(AMBER, dark));
        // A server on this PC keeps the data here; any other gets it.
        let server = |url: &str| {
            if crate::net::is_on_this_pc(url) {
                (t!("settings-data-on-pc"), weak)
            } else {
                let host = crate::net::host(url);
                (t!("settings-data-sent", host = host), warn)
            }
        };
        let history = if self.library.store().is_encrypted() {
            t!("settings-data-history-encrypted")
        } else {
            t!("settings-data-history-local")
        };
        let search = match d.search_backend() {
            EmbedBackend::None => (t!("settings-data-off"), weak),
            EmbedBackend::Server(s) => server(&s.base_url),
        };
        let reader = match d.text_reader() {
            TextReader::Off => (t!("settings-data-off"), weak),
            TextReader::Windows => (t!("settings-data-windows"), weak),
            TextReader::Server(r) => server(&r.base_url),
        };
        let updates = if d.update_check {
            t!("settings-data-updates-on")
        } else {
            t!("settings-data-off")
        };
        for (label, (value, color)) in [
            (t!("settings-data-history"), (history, weak)),
            (t!("settings-data-search"), search),
            (t!("settings-data-reader"), reader),
            (t!("settings-data-updates"), (updates, weak)),
        ] {
            plain_setting(ui, &label, "", "", |ui| {
                lbl(ui, &value, size::BODY, color);
            });
        }
    }
}

/// The rules for copies that aren't recorded: one row each, then "Add a rule".
fn rules_section(ui: &mut Ui, d: &mut Config) {
    section(ui, &t!("settings-rules"));
    note(ui, &t!("settings-rules-help"));
    ui.add_space(4.0);
    let mut remove = None;
    for (index, rule) in d.rules.iter_mut().enumerate() {
        ui.push_id(index, |ui| {
            row(ui, |ui| {
                ui.add_space(10.0);
                let program = ui.add(
                    egui::TextEdit::singleline(&mut rule.app)
                        .hint_text(txt(&t!("settings-rule-program"), size::BODY, None))
                        .desired_width(150.0),
                );
                set_name(&program, &t!("settings-rule-program"));
                let pattern = ui.add(
                    egui::TextEdit::singleline(&mut rule.pattern)
                        .hint_text(txt(&t!("settings-rule-pattern"), size::BODY, None))
                        .desired_width(220.0),
                );
                set_name(&pattern, &t!("settings-rule-pattern"));
                tip(
                    toggle(ui, &mut rule.is_regex),
                    &t!("settings-rule-regex-help"),
                );
                lbl(
                    ui,
                    &t!("settings-rule-regex"),
                    size::CONTROL,
                    ui.visuals().text_color(),
                );
                far(ui, |ui| {
                    if icons::icon_button(ui, Icon::Delete, &t!("settings-rule-delete")).clicked() {
                        remove = Some(index);
                    }
                });
            });
            match rule.compile() {
                Err(crate::rules::Problem::Empty) => note(ui, &t!("settings-rule-needs")),
                Err(crate::rules::Problem::BadPattern(error)) => note(
                    ui,
                    &format!(
                        "⚠ {}",
                        t!("settings-rule-bad-pattern", error = error.as_str())
                    ),
                ),
                Ok(_) => {}
            }
        });
    }
    if let Some(index) = remove {
        d.rules.remove(index);
    }
    row(ui, |ui| {
        ui.add_space(10.0);
        if ui
            .button(txt(&t!("settings-rule-add"), size::CONTROL, None))
            .clicked()
        {
            d.rules.push(crate::rules::Rule::default());
        }
    });
}
