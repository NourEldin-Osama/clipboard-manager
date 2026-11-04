//! Settings, About: version and updates, where the data is, and resetting.

use super::*;

impl App {
    pub(super) fn about_page(&mut self, ui: &mut Ui, d: &mut Config) {
        self.updates_section(ui, d);
        self.storage_section(ui);
        self.reset_section(ui, d);
    }

    fn updates_section(&mut self, ui: &mut Ui, d: &mut Config) {
        let def = Config::default();
        section(ui, &t!("settings-updates"));
        let release_build = crate::update::build().is_some();
        setting_for(
            ui,
            &t!("settings-update-check"),
            &t!("settings-update-check-hint"),
            &t!("settings-update-check-help"),
            &mut d.update_check,
            def.update_check,
            toggle,
        );
        setting_for(
            ui,
            &t!("settings-update-download"),
            &t!("settings-update-download-hint"),
            &t!("settings-update-download-help"),
            &mut d.update_download,
            def.update_download,
            toggle,
        );
        let mut check = false;
        plain_setting(
            ui,
            &t!("settings-version"),
            &t!("settings-version-hint"),
            "",
            |ui| {
                if release_build
                    && ui
                        .add_enabled(
                            self.update_rx.is_none(),
                            egui::Button::new(txt(&t!("settings-check-now"), size::CONTROL, None)),
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
                // The version comes after the status, so it is laid out first
                // and the status is cut to the room it leaves.
                let version = txt(&crate::update::current_label(), size::CONTROL, None)
                    .into_galley(
                        ui,
                        Some(egui::TextWrapMode::Extend),
                        f32::INFINITY,
                        egui::FontSelection::Default,
                    );
                if !status.is_empty() {
                    let room =
                        ui.available_width() - version.size().x - 2.0 * ui.spacing().item_spacing.x;
                    cut_line(
                        ui,
                        &status,
                        room.max(0.0),
                        size::SMALL,
                        weak,
                        Sense::hover(),
                    );
                }
                ui.label(version);
            },
        );
        if check {
            let ctx = ui.ctx().clone();
            self.update_hidden = false;
            self.check_for_update(true, &ctx);
        }
    }

    fn storage_section(&mut self, ui: &mut Ui) {
        section(ui, &t!("settings-storage"));
        let root = self.paths.root.clone();
        plain_setting(
            ui,
            &t!("settings-data-folder"),
            &t!("settings-data-folder-hint"),
            "",
            |ui| {
                if ui
                    .button(txt(&t!("settings-open"), size::BODY, None))
                    .clicked()
                {
                    if let Err(e) = reveal(&root) {
                        self.set_error(log_error!("error-opening-folder", error = e.to_string()));
                    }
                }
                ui.add(
                    egui::Label::new(
                        RichText::new(root.display().to_string())
                            .size(size::SMALL)
                            .color(ui.visuals().weak_text_color()),
                    )
                    .truncate(),
                );
            },
        );
        let log = root.join("clipboard-manager.log");
        plain_setting(
            ui,
            &t!("settings-log-file"),
            &t!("settings-log-file-hint"),
            &t!("settings-log-file-help"),
            |ui| {
                if ui
                    .button(txt(&t!("settings-show"), size::BODY, None))
                    .clicked()
                {
                    if let Err(e) = reveal(&log) {
                        self.set_error(log_error!("error-opening-folder", error = e.to_string()));
                    }
                }
            },
        );
        let items = self.library.items();
        let images = items.iter().filter(|i| i.kind == Kind::Image).count();
        let pinned = items.iter().filter(|i| i.pinned).count();
        plain_setting(ui, &t!("settings-stored"), "", "", |ui| {
            ui.label(txt(
                &t!(
                    "settings-stored-counts",
                    count = items.len(),
                    images = images,
                    pinned = pinned
                ),
                size::CONTROL,
                None,
            ));
        });
    }

    fn reset_section(&mut self, ui: &mut Ui, d: &mut Config) {
        section(ui, &t!("settings-reset-section"));
        let mut reset = false;
        let confirm = self.confirm;
        plain_setting(
            ui,
            &t!("settings-reset-all"),
            &t!("settings-reset-all-hint"),
            "",
            |ui| {
                if confirm == Some(Confirm::ResetSettings) {
                    if ui
                        .add(
                            egui::Button::new(txt(
                                &t!("settings-reset-everything"),
                                size::BODY,
                                Some(Color32::WHITE),
                            ))
                            .fill(button_fill(RED)),
                        )
                        .clicked()
                    {
                        reset = true;
                    }
                    if ui
                        .button(txt(&t!("common-cancel"), size::BODY, None))
                        .clicked()
                    {
                        self.confirm = None;
                    }
                } else if ui
                    .add_enabled(
                        *d != d.reset(),
                        egui::Button::new(txt(&t!("settings-reset"), size::BODY, None)),
                    )
                    .clicked()
                {
                    self.confirm = Some(Confirm::ResetSettings);
                }
            },
        );
        if reset {
            self.confirm = None;
            *d = d.reset();
        }
    }
}
