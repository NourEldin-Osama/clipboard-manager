//! Settings, Clipboard: what is kept, and for how long.

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

impl App {
    pub(super) fn clipboard_page(&mut self, ui: &mut Ui, d: &mut Config) {
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
            &t!("settings-merge-duplicates"),
            &t!("settings-merge-duplicates-help"),
            &mut d.dedupe,
            def.dedupe,
            toggle,
        );
        section(ui, &t!("settings-how-long"));
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
                let menu = egui::ComboBox::from_id_salt("history")
                    .selected_text(txt(&label(*max_items), size::BODY, None))
                    .width(140.0)
                    .show_ui(ui, |ui| {
                        for n in HISTORY_SIZES {
                            ui.selectable_value(max_items, n, txt(&label(n), size::BODY, None));
                        }
                    });
                set_choice_name(&menu.response, &row_title(ui), &label(*max_items));
            },
        );
        setting_for(
            ui,
            &t!("settings-delete-older"),
            &t!("settings-delete-older-help"),
            &mut d.delete_after_days,
            def.delete_after_days,
            |ui, days| {
                let menu = egui::ComboBox::from_id_salt("age")
                    .selected_text(txt(&age_label(*days), size::BODY, None))
                    .width(140.0)
                    .show_ui(ui, |ui| {
                        for n in AGES {
                            ui.selectable_value(days, n, txt(&age_label(n), size::BODY, None));
                        }
                    });
                set_choice_name(&menu.response, &row_title(ui), &age_label(*days));
            },
        );
        self.clear_history_setting(ui);
    }

    fn clear_history_setting(&mut self, ui: &mut Ui) {
        let mut clear = false;
        let confirm = self.confirm;
        let clip = self.library.clearable_count();
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
                                size::BODY,
                                Some(Color32::WHITE),
                            ))
                            .fill(button_fill(RED)),
                        )
                        .clicked()
                    {
                        clear = true;
                    }
                    if ui
                        .button(txt(&t!("common-cancel"), size::BODY, None))
                        .clicked()
                    {
                        self.confirm = None;
                    }
                } else if ui
                    .add_enabled(
                        clip > 0,
                        egui::Button::new(txt(
                            &t!("settings-clear"),
                            size::BODY,
                            Some(text_shade(RED, ui.visuals().dark_mode)),
                        )),
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
    }
}
