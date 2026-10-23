//! Settings, General: language, clock, start at sign-in, tray and updates.

use super::*;

pub(super) fn general_settings(ui: &mut Ui, d: &mut Config) {
    let def = Config::default();
    section(ui, &t!("settings-language-and-time"));
    // Each language is named in itself, so it can be found whatever is showing.
    setting_for(
        ui,
        &t!("settings-language"),
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
    section(ui, &t!("settings-startup"));
    setting_for(
        ui,
        &t!("settings-start-at-sign-in"),
        &t!("settings-start-at-sign-in-help"),
        &mut d.start_at_login,
        def.start_at_login,
        toggle,
    );
    section(ui, &t!("settings-tray-section"));
    setting_for(
        ui,
        &t!("settings-tray"),
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
        self.updates_section(ui, d);
    }

    pub(super) fn updates_section(&mut self, ui: &mut Ui, d: &mut Config) {
        let def = Config::default();
        section(ui, &t!("settings-updates"));
        let release_build = crate::update::build().is_some();
        setting_for(
            ui,
            &t!("settings-update-check"),
            &t!("settings-update-check-help"),
            &mut d.update_check,
            def.update_check,
            toggle,
        );
        setting_for(
            ui,
            &t!("settings-update-download"),
            &t!("settings-update-download-help"),
            &mut d.update_download,
            def.update_download,
            toggle,
        );
        let mut check = false;
        plain_setting(
            ui,
            &t!("settings-version"),
            &t!("settings-version-help"),
            |ui| {
                if release_build
                    && ui
                        .add_enabled(
                            self.update_rx.is_none(),
                            egui::Button::new(txt(&t!("settings-check-now"), 13.0, None)),
                        )
                        .clicked()
                {
                    check = true;
                }
                let weak = ui.visuals().weak_text_color();
                let status = match &self.update {
                    UpdateState::Checking => t!("settings-update-checking"),
                    UpdateState::UpToDate => t!("settings-update-current"),
                    UpdateState::Available(r) => t!("update-available", version = r.label()),
                    UpdateState::Downloading(r, _) => {
                        t!("update-downloading", version = r.label())
                    }
                    UpdateState::Ready(r, _) => t!("update-ready", version = r.label()),
                    UpdateState::Failed { error, .. } => {
                        t!("update-failed", error = error.as_str())
                    }
                    UpdateState::Idle if !release_build => {
                        t!("settings-updates-off-dev")
                    }
                    UpdateState::Idle => String::new(),
                };
                if !status.is_empty() {
                    ui.add(egui::Label::new(txt(&status, 12.5, Some(weak))).truncate());
                }
                ui.label(txt(&crate::update::current_label(), 13.0, None));
            },
        );
        if check {
            let ctx = ui.ctx().clone();
            self.update_hidden = false;
            self.check_for_update(true, &ctx);
        }
    }
}
