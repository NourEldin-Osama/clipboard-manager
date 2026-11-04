//! Settings, General: language, clock, start at sign-in and the tray.

use super::*;

pub(super) fn general_settings(ui: &mut Ui, d: &mut Config) {
    let def = Config::default();
    section(ui, &t!("settings-language-and-time"));
    // Each language is named in itself, so it can be found whatever is showing.
    setting_for(
        ui,
        &t!("settings-language"),
        &t!("settings-language-hint"),
        &t!("settings-language-help"),
        &mut d.language,
        def.language,
        |ui, language| {
            segmented(
                ui,
                language,
                &[
                    (Language::English, "English"),
                    (Language::Arabic, "العربية"),
                ],
            )
        },
    );
    setting_for(
        ui,
        &t!("settings-time-format"),
        &t!("settings-time-format-hint"),
        &t!("settings-time-format-help"),
        &mut d.time_format,
        def.time_format,
        |ui, time_format| {
            segmented(
                ui,
                time_format,
                &[
                    (TimeFormat::H24, t!("settings-time-24h")),
                    (TimeFormat::H12, t!("settings-time-12h")),
                ],
            )
        },
    );
    section(ui, &t!("settings-start-and-tray"));
    setting_for(
        ui,
        &t!("settings-start-at-sign-in"),
        &t!("settings-start-at-sign-in-hint"),
        &t!("settings-start-at-sign-in-help"),
        &mut d.start_at_login,
        def.start_at_login,
        toggle,
    );
    setting_for(
        ui,
        &t!("settings-tray"),
        &t!("settings-tray-hint"),
        &t!("settings-tray-help"),
        &mut d.tray,
        def.tray,
        toggle,
    );
    note(ui, &t!("settings-startup-note"));
}

impl App {
    pub(super) fn general_page(&mut self, ui: &mut Ui, d: &mut Config) {
        general_settings(ui, d);
    }
}
