//! Settings, Clipboard: what is recorded and how much history is kept.

use super::*;

pub(super) const HISTORY_SIZES: [usize; 5] = [500, 1_000, 5_000, 20_000, 100_000];

pub(super) fn clipboard_page(ui: &mut Ui, d: &mut Config) {
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
}
