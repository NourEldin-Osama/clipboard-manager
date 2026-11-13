//! Settings, General: language, clock, start at sign-in and the tray.

use super::*;

pub(super) fn general_settings(ui: &mut Ui, d: &mut Config) {
    let def = Config::default();
    section(ui, &t!("settings-language-and-time"), |ui| {
        // Each language is named in itself, so it can be found whatever is showing.
        setting_for(
            ui,
            &t!("settings-language"),
            &t!("settings-language-hint"),
            "",
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
            "",
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
    });
    section(ui, &t!("settings-start-and-tray"), |ui| {
        setting_for(
            ui,
            &t!("settings-start-at-sign-in"),
            &t!("settings-start-at-sign-in-hint"),
            "",
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
        // Only with both on does the app start hidden in the tray.
        if d.start_at_login && d.tray {
            note(ui, &t!("settings-startup-note"));
        }
    });
}

impl App {
    pub(super) fn general_page(&mut self, ui: &mut Ui, d: &mut Config) {
        general_settings(ui, d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether the note about starting hidden shows, with start at sign-in
    /// and the tray on or off.
    fn startup_note_shows(start_at_login: bool, tray: bool) -> bool {
        let ctx = egui::Context::default();
        let mut d = Config {
            start_at_login,
            tray,
            ..Default::default()
        };
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| general_settings(ui, &mut d));
        // The font atlas upload no renderer takes here.
        output.textures_delta.clear();
        let note = t!("settings-startup-note");
        output.shapes.iter().any(|clipped| {
            matches!(&clipped.shape, egui::Shape::Text(text) if text.galley.text() == note)
        })
    }

    #[test]
    fn the_startup_note_shows_only_with_both_switches_on() {
        assert!(startup_note_shows(true, true));
        assert!(!startup_note_shows(true, false));
        assert!(!startup_note_shows(false, true));
        assert!(!startup_note_shows(false, false));
    }
}
