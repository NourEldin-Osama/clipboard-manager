//! Settings, Clipboard: what is recorded and how much history is kept.

use super::*;

pub(super) const HISTORY_SIZES: [usize; 5] = [500, 1_000, 5_000, 20_000, 100_000];

/// The "Delete items older than" choices, in days; 0 is off.
const AGES: [u32; 6] = [0, 1, 7, 30, 90, 365];

fn age_label(days: u32) -> String {
    match days {
        0 => t!("settings-age-off"),
        1 => t!("settings-age-day"),
        7 => t!("settings-age-week"),
        30 => t!("settings-age-month"),
        90 => t!("settings-age-three-months"),
        365 => t!("settings-age-year"),
        _ => t!("settings-age-days", count = days),
    }
}

/// The readers offered, in order, each with fresh settings.
fn reader_choices() -> Vec<TextReader> {
    let mut choices = Vec::new();
    if cfg!(windows) {
        choices.push(TextReader::Windows);
    }
    choices.push(TextReader::BuiltIn);
    choices.push(TextReader::Server(ReaderServer::default()));
    choices
}

fn reader_label(reader: &TextReader) -> String {
    match reader {
        TextReader::Windows => t!("settings-reader-windows"),
        TextReader::BuiltIn => t!("settings-reader-built-in"),
        TextReader::Server(_) => t!("settings-reader-server"),
    }
}

/// The same kind of reader, whatever a server's settings.
fn same_kind(a: &TextReader, b: &TextReader) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

impl App {
    pub(super) fn clipboard_page(&mut self, ui: &mut Ui, d: &mut Config, apply_reader: &mut bool) {
        let def = Config::default();
        section(ui, &t!("settings-recording"));
        setting_for(
            ui,
            &t!("settings-record"),
            &t!("settings-record-help"),
            &mut d.monitor_clipboard,
            def.monitor_clipboard,
            toggle,
        );
        setting_for(
            ui,
            &t!("settings-skip-secrets"),
            &t!("settings-skip-secrets-help"),
            &mut d.skip_secrets,
            def.skip_secrets,
            toggle,
        );
        setting_for(
            ui,
            &t!("settings-merge-duplicates"),
            &t!("settings-merge-duplicates-help"),
            &mut d.dedupe,
            def.dedupe,
            toggle,
        );
        setting_for(
            ui,
            &t!("settings-read-images"),
            &t!("settings-read-images-help"),
            &mut d.ocr_images,
            def.ocr_images,
            toggle,
        );
        if d.ocr_images {
            self.reader_settings(ui, d, apply_reader);
        }
        section(ui, &t!("settings-after-copying"));
        setting_for(
            ui,
            &t!("settings-hide-window"),
            &t!("settings-hide-window-help"),
            &mut d.minimize_after_copy,
            def.minimize_after_copy,
            toggle,
        );
        section(ui, &t!("settings-history-size"));
        setting_for(
            ui,
            &t!("settings-keep-up-to"),
            &t!("settings-keep-up-to-help"),
            &mut d.max_items,
            def.max_items,
            |ui, max_items| {
                let label = |n: usize| {
                    t!(
                        "settings-history-items",
                        size = n
                            .to_string()
                            .as_bytes()
                            .rchunks(3)
                            .rev()
                            .map(|c| std::str::from_utf8(c).unwrap())
                            .collect::<Vec<_>>()
                            .join(",")
                    )
                };
                egui::ComboBox::from_id_salt("history")
                    .selected_text(txt(&label(*max_items), 14.0, None))
                    .width(140.0)
                    .show_ui(ui, |ui| {
                        for n in HISTORY_SIZES {
                            ui.selectable_value(max_items, n, txt(&label(n), 14.0, None));
                        }
                    });
            },
        );
        setting_for(
            ui,
            &t!("settings-delete-older"),
            &t!("settings-delete-older-help"),
            &mut d.delete_after_days,
            def.delete_after_days,
            |ui, days| {
                egui::ComboBox::from_id_salt("age")
                    .selected_text(txt(&age_label(*days), 14.0, None))
                    .width(140.0)
                    .show_ui(ui, |ui| {
                        for n in AGES {
                            ui.selectable_value(days, n, txt(&age_label(n), 14.0, None));
                        }
                    });
            },
        );
        rules_section(ui, d);
    }

    /// What reads the text in images, and a server's settings. A change waits
    /// for Apply, which first asks about a server outside this PC.
    fn reader_settings(&mut self, ui: &mut Ui, d: &mut Config, apply_reader: &mut bool) {
        let applied = self.cfg.text_reader.clone();
        plain_setting(
            ui,
            &t!("settings-reader"),
            &t!("settings-reader-help"),
            |ui| {
                egui::ComboBox::from_id_salt("text-reader")
                    .selected_text(txt(&reader_label(&d.text_reader), 14.0, None))
                    .width(140.0)
                    .show_ui(ui, |ui| {
                        for choice in reader_choices() {
                            let chosen = same_kind(&d.text_reader, &choice);
                            let label = txt(&reader_label(&choice), 14.0, None);
                            if ui.selectable_label(chosen, label).clicked() && !chosen {
                                // Back to the server in use: its settings, not new ones.
                                d.text_reader = if same_kind(&applied, &choice) {
                                    applied.clone()
                                } else {
                                    choice
                                };
                            }
                        }
                    });
            },
        );
        if let TextReader::Server(s) = &mut d.text_reader {
            reader_server_settings(ui, s, &mut self.model_list.show_key);
        }
        let send_to = if self.confirm == Some(Confirm::SendImagesToServer) {
            self.unconfirmed_reader(d)
        } else {
            None
        };
        if let Some(host) = send_to {
            ui.add_space(8.0);
            Frame::new()
                .inner_margin(Margin::symmetric(10, 0))
                .show(ui, |ui| {
                    let strong = ui.visuals().strong_text_color();
                    let question = t!("settings-reader-send-question", host = host.as_str());
                    super::bidi::paragraph(ui, &question, egui::FontId::proportional(13.5), strong);
                });
            ui.add_space(4.0);
            row(ui, |ui| {
                ui.add_space(10.0);
                if primary_button(ui, &t!("settings-server-send")).clicked() {
                    if let TextReader::Server(s) = &d.text_reader {
                        let url = s.base_url.clone();
                        d.allow_server(&url);
                    }
                    *apply_reader = true;
                    self.confirm = None;
                }
                if ui
                    .button(txt(&t!("settings-server-send-cancel"), 13.0, None))
                    .clicked()
                {
                    d.text_reader = applied.clone();
                    self.confirm = None;
                }
            });
        } else if d.text_reader != applied {
            if self.confirm == Some(Confirm::SendImagesToServer) {
                self.confirm = None;
            }
            ui.add_space(8.0);
            row(ui, |ui| {
                ui.add_space(10.0);
                if primary_button(ui, &t!("settings-reader-apply")).clicked() {
                    *apply_reader = true;
                }
                if ui
                    .button(txt(&t!("settings-discard-changes"), 13.0, None))
                    .clicked()
                {
                    d.text_reader = applied.clone();
                }
                lbl(
                    ui,
                    &t!("settings-reader-new-images"),
                    12.5,
                    ui.visuals().weak_text_color(),
                );
            });
        }
    }
}

/// A text reader server's address, key, model and prompt.
fn reader_server_settings(ui: &mut Ui, s: &mut ReaderServer, show_key: &mut bool) {
    plain_setting(
        ui,
        &t!("settings-server-address"),
        &t!("settings-reader-address-help"),
        |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut s.base_url)
                    .hint_text("http://127.0.0.1:8080/v1")
                    .desired_width(300.0),
            );
        },
    );
    if !s.base_url.trim().is_empty() && !crate::net::is_on_this_pc(&s.base_url) {
        let host = crate::net::host(&s.base_url);
        let text = if s
            .base_url
            .trim()
            .to_ascii_lowercase()
            .starts_with("http://")
        {
            t!("settings-reader-remote-http", host = host)
        } else {
            t!("settings-reader-remote", host = host)
        };
        note(ui, &format!("⚠ {text}"));
    }
    plain_setting(
        ui,
        &t!("settings-server-key"),
        &t!("settings-server-key-help"),
        |ui| {
            let (eye, label) = if *show_key {
                (Icon::EyeOff, t!("settings-server-hide-key"))
            } else {
                (Icon::Eye, t!("settings-server-show-key"))
            };
            if icons::icon_button_ex(ui, eye, &label, Vec2::splat(28.0), 16.0, None, None).clicked()
            {
                *show_key = !*show_key;
            }
            ui.add(
                egui::TextEdit::singleline(&mut s.api_key)
                    .password(!*show_key)
                    .hint_text(txt(&t!("settings-server-optional"), 14.0, None))
                    .desired_width(300.0),
            );
        },
    );
    plain_setting(
        ui,
        &t!("settings-server-model"),
        &t!("settings-reader-model-help"),
        |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut s.model)
                    .hint_text(txt(&t!("settings-server-optional"), 14.0, None))
                    .desired_width(300.0),
            );
        },
    );
    plain_setting(
        ui,
        &t!("settings-reader-prompt"),
        &t!("settings-reader-prompt-help"),
        |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut s.prompt)
                    .desired_rows(2)
                    .desired_width(300.0),
            );
        },
    );
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
                ui.add(
                    egui::TextEdit::singleline(&mut rule.app)
                        .hint_text(txt(&t!("settings-rule-program"), 14.0, None))
                        .desired_width(150.0),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut rule.pattern)
                        .hint_text(txt(&t!("settings-rule-pattern"), 14.0, None))
                        .desired_width(220.0),
                );
                tip(
                    toggle(ui, &mut rule.is_regex),
                    &t!("settings-rule-regex-help"),
                );
                lbl(
                    ui,
                    &t!("settings-rule-regex"),
                    13.0,
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
            .button(txt(&t!("settings-rule-add"), 13.0, None))
            .clicked()
        {
            d.rules.push(crate::rules::Rule::default());
        }
    });
}
