//! Settings, General: language, clock, start at sign-in, tray and updates.

use super::*;

pub(super) fn general_settings(ui: &mut Ui, d: &mut Config) {
    let def = Config::default();
    section(ui, "Language and time");
    // Each language is named in itself, so it can be found whatever is showing.
    if setting(
        ui,
        "Language",
        "The language of the app. Arabic also lays the window out right to left.",
        d.language != def.language,
        |ui| {
            segmented(
                ui,
                &mut d.language,
                &[
                    (Language::English, "English"),
                    (Language::Arabic, "العربية"),
                ],
            );
        },
    ) {
        d.language = def.language;
    }
    if setting(
        ui,
        "Time format",
        "How times are shown. They're stored in UTC and shown in your time zone.",
        d.time_format != def.time_format,
        |ui| {
            segmented(
                ui,
                &mut d.time_format,
                &[(TimeFormat::H24, "24-hour"), (TimeFormat::H12, "12-hour")],
            );
        },
    ) {
        d.time_format = def.time_format;
    }
    section(ui, "Startup");
    if setting(ui, "Start when you sign in", "Opens the app each time you sign in to Windows. It starts hidden: in the system tray when that's on, otherwise minimised.", d.start_at_login != def.start_at_login, |ui| {
        toggle(ui, &mut d.start_at_login);
    }) {
        d.start_at_login = def.start_at_login;
    }
    section(ui, "System tray");
    if setting(ui, "Keep running in the system tray", "Shows an icon in the notification area. Closing or minimising the window hides it there and history keeps recording. Click the icon to open; right-click it to quit.", d.tray != def.tray, |ui| {
        toggle(ui, &mut d.tray);
    }) {
        d.tray = def.tray;
    }
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
        if setting(ui, "Check for updates automatically", "Looks for a new version when the app starts and once a day. Only the version file on GitHub is fetched; nothing about you or your clipboard is sent.", d.update_check != def.update_check, |ui| {
            toggle(ui, &mut d.update_check);
        }) {
            d.update_check = def.update_check;
        }
        if setting(ui, "Download updates automatically", "On: a new version downloads in the background and installs when you restart or quit. Off: you're asked before anything is downloaded.", d.update_download != def.update_download, |ui| {
            toggle(ui, &mut d.update_download);
        }) {
            d.update_download = def.update_download;
        }
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
