//! Settings, Data and privacy: data folder, log, clearing and resetting.

use super::*;

impl App {
    pub(super) fn data_page(&mut self, ui: &mut Ui, d: &mut Config) {
        section(ui, "Storage");
        let root = self.paths.root.clone();
        plain_setting(
            ui,
            "Data folder",
            "History, images, models and settings are stored here, on this PC only.",
            |ui| {
                if ui.button(txt(tr("Open"), 14.0, None)).clicked() {
                    reveal(&root);
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
            "Log file",
            "A record of what the app did in this session. Attach it when reporting a problem.",
            |ui| {
                if ui.button(txt(tr("Show"), 14.0, None)).clicked() {
                    reveal(&log);
                }
            },
        );
        let items = self.library.items();
        let images = items.iter().filter(|i| i.kind == Kind::Image).count();
        let pinned = items.iter().filter(|i| i.pinned).count();
        let clip = self.library.clearable_count();
        plain_setting(ui, "Stored", "", |ui| {
            ui.label(txt(
                &trf(
                    "{n} items · {i} images · {p} pinned",
                    &[("n", &items.len()), ("i", &images), ("p", &pinned)],
                ),
                13.0,
                None,
            ));
        });

        section(ui, "Clean up");
        let mut clear = false;
        let confirm = self.confirm;
        plain_setting(
            ui,
            "Clear clipboard history",
            "Deletes copied items. Pinned and tagged items, and files you added, stay.",
            |ui| {
                if confirm == Some("clear") {
                    if ui
                        .add(
                            egui::Button::new(txt(
                                &trf("Delete {n} items", &[("n", &clip)]),
                                14.0,
                                Some(Color32::WHITE),
                            ))
                            .fill(RED),
                        )
                        .clicked()
                    {
                        clear = true;
                    }
                    if ui.button("Cancel").clicked() {
                        self.confirm = None;
                    }
                } else if ui
                    .add_enabled(
                        clip > 0,
                        egui::Button::new(txt(tr("Clear…"), 14.0, Some(RED))),
                    )
                    .clicked()
                {
                    self.confirm = Some("clear");
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
            "Reset all settings",
            "Puts every setting back to its default, including the search model.",
            |ui| {
                if confirm == Some("reset") {
                    if ui
                        .add(
                            egui::Button::new(txt(
                                tr("Reset everything"),
                                14.0,
                                Some(Color32::WHITE),
                            ))
                            .fill(RED),
                        )
                        .clicked()
                    {
                        reset = true;
                    }
                    if ui.button("Cancel").clicked() {
                        self.confirm = None;
                    }
                } else if ui
                    .add_enabled(
                        *d != Config::default(),
                        egui::Button::new(txt(tr("Reset…"), 14.0, None)),
                    )
                    .clicked()
                {
                    self.confirm = Some("reset");
                }
            },
        );
        if reset {
            self.confirm = None;
            *d = Config::default();
        }
        note(ui, "Nothing you copy is sent anywhere. Search models run on this PC or on a server you choose. The update check only downloads the version file from GitHub.");
    }
}
