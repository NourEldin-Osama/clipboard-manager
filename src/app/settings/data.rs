//! Settings, Data and privacy: data folder, log, clearing and resetting.

use super::*;

impl App {
    pub(super) fn data_page(&mut self, ui: &mut Ui, d: &mut Config) {
        section(ui, &t!("settings-storage"));
        let root = self.paths.root.clone();
        plain_setting(
            ui,
            &t!("settings-data-folder"),
            &t!("settings-data-folder-help"),
            |ui| {
                if ui.button(txt(&t!("settings-open"), 14.0, None)).clicked() {
                    if let Err(e) = reveal(&root) {
                        self.set_error(log_error!("error-opening-folder", error = e.to_string()));
                    }
                }
                ui.add(
                    egui::Label::new(
                        RichText::new(root.display().to_string())
                            .size(12.0)
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
            &t!("settings-log-file-help"),
            |ui| {
                if ui.button(txt(&t!("settings-show"), 14.0, None)).clicked() {
                    if let Err(e) = reveal(&log) {
                        self.set_error(log_error!("error-opening-folder", error = e.to_string()));
                    }
                }
            },
        );
        let items = self.library.items();
        let images = items.iter().filter(|i| i.kind == Kind::Image).count();
        let pinned = items.iter().filter(|i| i.pinned).count();
        let clip = self.library.clearable_count();
        plain_setting(ui, &t!("settings-stored"), "", |ui| {
            ui.label(txt(
                &t!(
                    "settings-stored-counts",
                    count = items.len(),
                    images = images,
                    pinned = pinned
                ),
                13.0,
                None,
            ));
        });

        section(ui, &t!("settings-clean-up"));
        let mut clear = false;
        let confirm = self.confirm;
        plain_setting(
            ui,
            &t!("settings-clear-history"),
            &t!("settings-clear-history-help"),
            |ui| {
                if confirm == Some(Confirm::ClearHistory) {
                    if ui
                        .add(
                            egui::Button::new(txt(
                                &t!("settings-delete-items", count = clip),
                                14.0,
                                Some(Color32::WHITE),
                            ))
                            .fill(RED),
                        )
                        .clicked()
                    {
                        clear = true;
                    }
                    if ui.button(t!("common-cancel")).clicked() {
                        self.confirm = None;
                    }
                } else if ui
                    .add_enabled(
                        clip > 0,
                        egui::Button::new(txt(&t!("settings-clear"), 14.0, Some(RED))),
                    )
                    .clicked()
                {
                    self.confirm = Some(Confirm::ClearHistory);
                }
            },
        );
        if clear {
            self.confirm = None;
            self.run(Command::ClearHistory);
        }
        let mut reset = false;
        let confirm = self.confirm;
        plain_setting(
            ui,
            &t!("settings-reset-all"),
            &t!("settings-reset-all-help"),
            |ui| {
                if confirm == Some(Confirm::ResetSettings) {
                    if ui
                        .add(
                            egui::Button::new(txt(
                                &t!("settings-reset-everything"),
                                14.0,
                                Some(Color32::WHITE),
                            ))
                            .fill(RED),
                        )
                        .clicked()
                    {
                        reset = true;
                    }
                    if ui.button(t!("common-cancel")).clicked() {
                        self.confirm = None;
                    }
                } else if ui
                    .add_enabled(
                        *d != Config::default(),
                        egui::Button::new(txt(&t!("settings-reset"), 14.0, None)),
                    )
                    .clicked()
                {
                    self.confirm = Some(Confirm::ResetSettings);
                }
            },
        );
        if reset {
            self.confirm = None;
            *d = Config::default();
        }
        note(ui, &t!("settings-privacy-note"));
        if let EmbedBackend::Server(s) = &self.cfg.backend {
            if !crate::net::is_on_this_pc(&s.base_url) {
                let host = crate::net::host(&s.base_url);
                note(
                    ui,
                    &format!("⚠ {}", t!("settings-privacy-remote", host = host)),
                );
            }
        }
    }
}
