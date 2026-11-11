//! Settings, About: version and updates, where the data is, and resetting.

use super::*;

/// The download's progress line in the Version row.
const PROGRESS_BAR_WIDTH: f32 = 96.0;

impl App {
    pub(super) fn about_page(&mut self, ui: &mut Ui, d: &mut Config) {
        self.updates_section(ui, d);
        self.storage_section(ui);
        self.reset_section(ui, d);
    }

    fn updates_section(&mut self, ui: &mut Ui, d: &mut Config) {
        let def = Config::default();
        let release_build = crate::update::build().is_some();
        let mut check = false;
        let mut action = None;
        section(ui, &t!("settings-updates"), |ui| {
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
            plain_setting(
                ui,
                &t!("settings-version"),
                &t!("settings-version-hint"),
                "",
                |ui| {
                    // From the far end: the update's buttons, the same as its
                    // message's (D65), then its progress and its state.
                    let weak = ui.visuals().weak_text_color();
                    // The next step in the accent's color.
                    let button = |ui: &mut Ui, label: &str, main: bool| {
                        let text = txt(label, size::CONTROL, main.then_some(Color32::WHITE));
                        if main {
                            ui.add(egui::Button::new(text).fill(button_fill(accent())))
                                .react(ui)
                                .clicked()
                        } else {
                            ui.button(text).clicked()
                        }
                    };
                    let whats_new = |ui: &mut Ui| {
                        ui.link(txt(&t!("update-whats-new"), size::CONTROL, None))
                            .clicked()
                    };
                    let mut red = false;
                    let status = match &self.update {
                        UpdateState::Available(r) => {
                            if button(ui, &t!("update-download"), true) {
                                action = Some(UpdateAction::Download);
                            }
                            if whats_new(ui) {
                                action = Some(UpdateAction::WhatsNew);
                            }
                            t!("update-available", version = r.label())
                        }
                        UpdateState::Downloading(r, p) => {
                            let (done, total) = download_amounts(r, p);
                            if done >= total {
                                slow_spinner(ui, 14.0, Some(weak));
                                t!("update-verifying", version = r.label())
                            } else {
                                if button(ui, &t!("common-cancel"), false) {
                                    action = Some(UpdateAction::Cancel);
                                }
                                let amount =
                                    t!("update-progress", done = done >> 20, total = total >> 20);
                                ui.label(txt(&amount, size::SMALL, Some(weak)));
                                let (bar, _) = ui.allocate_exact_size(
                                    Vec2::new(PROGRESS_BAR_WIDTH, PROGRESS_LINE),
                                    Sense::hover(),
                                );
                                let done = done as f32 / total as f32;
                                paint_progress(ui.painter(), bar, done, ui.visuals().dark_mode);
                                match p.retry_in() {
                                    Some(seconds) => t!("download-retrying", seconds = seconds),
                                    None => t!("update-downloading", version = r.label()),
                                }
                            }
                        }
                        UpdateState::Ready(r, _) => {
                            if button(ui, &t!("update-restart"), true) {
                                action = Some(UpdateAction::Restart);
                            }
                            if whats_new(ui) {
                                action = Some(UpdateAction::WhatsNew);
                            }
                            t!("update-ready", version = r.label())
                        }
                        UpdateState::Failed { error, .. } => {
                            if button(ui, &t!("update-try-again"), false) {
                                action = Some(UpdateAction::TryAgain);
                            }
                            red = true;
                            t!("update-failed", error = error.as_str())
                        }
                        state => {
                            if release_build
                                && ui
                                    .add_enabled(
                                        self.update_rx.is_none(),
                                        egui::Button::new(txt(
                                            &t!("settings-check-now"),
                                            size::CONTROL,
                                            None,
                                        )),
                                    )
                                    .clicked()
                            {
                                check = true;
                            }
                            match state {
                                UpdateState::Checking => t!("settings-update-checking"),
                                UpdateState::UpToDate => t!("settings-update-current"),
                                _ if !release_build => t!("settings-updates-off-dev"),
                                _ => String::new(),
                            }
                        }
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
                        let room = ui.available_width()
                            - version.size().x
                            - 2.0 * ui.spacing().item_spacing.x;
                        let color = if red {
                            text_shade(RED, ui.visuals().dark_mode)
                        } else {
                            weak
                        };
                        cut_line(
                            ui,
                            &status,
                            room.max(0.0),
                            size::SMALL,
                            color,
                            Sense::hover(),
                        );
                    }
                    ui.label(version);
                },
            );
        });
        let ctx = ui.ctx().clone();
        if check {
            self.update_hidden = false;
            self.check_for_update(true, &ctx);
        }
        if let Some(action) = action {
            self.update_action(action, &ctx);
        }
    }

    /// What a bug report needs, without addresses, keys or copies: the
    /// version, Windows' version, the main settings and the log's last
    /// lines. In English, like the log.
    fn bug_report_details(&self) -> String {
        use std::fmt::Write;
        let c = &self.cfg;
        let mut out = String::new();
        let build = crate::update::build().map_or_else(String::new, |b| format!(" (build {b})"));
        let _ = writeln!(out, "Clipboard Manager {}{build}", crate::update::VERSION);
        let _ = writeln!(out, "{}", crate::system::windows_version());
        let _ = writeln!(
            out,
            "Language {:?}, theme {:?}, dark panels {:?}, view {:?}",
            c.language, c.theme, c.dark_panels, c.view
        );
        // Each engine with the kind of server it uses, never its address.
        let kind = |at: Option<crate::config::ModelRef>| {
            at.and_then(|at| c.servers.iter().find(|s| s.id == at.server))
                .map_or_else(String::new, |s| format!(" on {:?}", s.kind))
        };
        let _ = writeln!(
            out,
            "Search {:?}{}, reading {:?}{}",
            c.search_engine,
            kind(c.search_model),
            c.read_engine,
            kind(c.read_model)
        );
        let _ = writeln!(
            out,
            "Recording {}, skip secrets {}, notes on the screen {}, tray {}",
            c.monitor_clipboard, c.skip_secrets, c.screen_notes, c.tray
        );
        let log = std::fs::read_to_string(self.paths.root.join("clipboard-manager.log"))
            .unwrap_or_default();
        let lines: Vec<&str> = log.lines().collect();
        let from = lines.len().saturating_sub(100);
        let _ = writeln!(out, "\nThe log's last {} lines:", lines.len() - from);
        for line in &lines[from..] {
            let _ = writeln!(out, "{line}");
        }
        out
    }

    fn storage_section(&mut self, ui: &mut Ui) {
        let root = self.paths.root.clone();
        section(ui, &t!("settings-storage"), |ui| {
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
                            self.set_error(log_error!(
                                "error-opening-folder",
                                error = e.to_string()
                            ));
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
                            self.set_error(log_error!(
                                "error-opening-folder",
                                error = e.to_string()
                            ));
                        }
                    }
                },
            );
            let mut copy = false;
            plain_setting(
                ui,
                &t!("settings-bug-report"),
                &t!("settings-bug-report-hint"),
                "",
                |ui| {
                    copy = ui
                        .button(txt(&t!("settings-bug-report-copy"), size::BODY, None))
                        .clicked();
                },
            );
            if copy {
                let details = self.bug_report_details();
                self.run(Command::CopyText(details));
            }
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
        });
    }

    fn reset_section(&mut self, ui: &mut Ui, d: &mut Config) {
        let mut reset = false;
        let confirm = self.confirm;
        section(ui, &t!("settings-reset-section"), |ui| {
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
                            .react(ui)
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
        });
        if reset {
            self.confirm = None;
            *d = d.reset();
        }
    }
}
