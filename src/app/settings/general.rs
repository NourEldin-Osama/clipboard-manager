//! Settings, General: language, clock, start at sign-in, tray and updates.

use super::*;

pub(super) fn general_settings(ui: &mut Ui, d: &mut Config) {
    let def = Config::default();
    section(ui, "Language and time");
    // Each language is named in itself, so it can be found whatever is showing.
    setting_for(
        ui,
        "Language",
        "The language of the app. Arabic also lays the window out right to left.",
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
        "Time format",
        "How times are shown. They're stored in UTC and shown in your time zone.",
        &mut d.time_format,
        def.time_format,
        |ui, time_format| {
            segmented(
                ui,
                time_format,
                &[(TimeFormat::H24, "24-hour"), (TimeFormat::H12, "12-hour")],
            )
        },
    );
    section(ui, "Startup");
    setting_for(ui, "Start when you sign in", "Opens the app each time you sign in to Windows. It starts hidden: in the system tray when that's on, otherwise minimised.", &mut d.start_at_login, def.start_at_login, toggle);
    section(ui, "System tray");
    setting_for(ui, "Keep running in the system tray", "Shows an icon in the notification area. Closing or minimising the window hides it there and history keeps recording. Click the icon to open; right-click it to quit.", &mut d.tray, def.tray, toggle);
    note(
        ui,
        "With both on, the app starts quietly in the tray each time you sign in.",
    );
}

impl App {
    pub(super) fn general_page(&mut self, ui: &mut Ui, d: &mut Config) {
        general_settings(ui, d);
        self.updates_section(ui, d);
    }

    pub(super) fn updates_section(&mut self, ui: &mut Ui, d: &mut Config) {
        let def = Config::default();
        section(ui, "Updates");
        let release_build = crate::update::build().is_some();
        setting_for(ui, "Check for updates automatically", "Looks for a new version when the app starts and once a day. Only the version file on GitHub is fetched; nothing about you or your clipboard is sent.", &mut d.update_check, def.update_check, toggle);
        setting_for(ui, "Download updates automatically", "On: a new version downloads in the background and installs when you restart or quit. Off: you're asked before anything is downloaded.", &mut d.update_download, def.update_download, toggle);
        let mut check = false;
        plain_setting(
            ui,
            "Version",
            "Every download is checked against this app's signing key before it's installed.",
            |ui| {
                if release_build
                    && ui
                        .add_enabled(
                            self.update_rx.is_none(),
                            egui::Button::new(txt(tr("Check now"), 13.0, None)),
                        )
                        .clicked()
                {
                    check = true;
                }
                let weak = ui.visuals().weak_text_color();
                let status = match &self.update {
                    UpdateState::Checking => tr("Checking…").to_string(),
                    UpdateState::UpToDate => tr("You're up to date").to_string(),
                    UpdateState::Available(r) => {
                        trf("Version {version} is available", &[("version", &r.label())])
                    }
                    UpdateState::Downloading(r, _) => {
                        trf("Downloading version {version}…", &[("version", &r.label())])
                    }
                    UpdateState::Ready(r, _) => {
                        trf("Version {version} is ready", &[("version", &r.label())])
                    }
                    UpdateState::Failed { error, .. } => {
                        trf("Update failed: {error}", &[("error", error)])
                    }
                    UpdateState::Idle if !release_build => {
                        tr("Updates are off in development builds").to_string()
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
