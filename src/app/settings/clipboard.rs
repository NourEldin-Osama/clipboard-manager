//! Settings, Clipboard: what is recorded and how much history is kept.

use super::*;

pub(super) const HISTORY_SIZES: [usize; 5] = [500, 1_000, 5_000, 20_000, 100_000];

pub(super) fn clipboard_page(ui: &mut Ui, d: &mut Config) {
    let def = Config::default();
    section(ui, "Recording");
    setting_for(ui, "Record clipboard history", "Save everything you copy. Turn off to pause; the pause button in the main window does the same.", &mut d.monitor_clipboard, def.monitor_clipboard, toggle);
    setting_for(ui, "Skip passwords and API keys", "Don't save copies that look like a password, API key, token or private key. You still can paste them normally.", &mut d.skip_secrets, def.skip_secrets, toggle);
    setting_for(
        ui,
        "Merge duplicate copies",
        "Copying something you already have moves it to the top instead of adding it again.",
        &mut d.dedupe,
        def.dedupe,
        toggle,
    );
    setting_for(
        ui,
        "Read text in images",
        "Find screenshots and photos by the words in them. Runs on this PC.",
        &mut d.ocr_images,
        def.ocr_images,
        toggle,
    );
    section(ui, "After copying from the list");
    setting_for(
        ui,
        "Hide the window",
        "After you copy an item, the window hides so you can paste right away.",
        &mut d.minimize_after_copy,
        def.minimize_after_copy,
        toggle,
    );
    section(ui, "History size");
    setting_for(ui, "Keep up to", "When there are more, the oldest are deleted. Pinned and tagged items, and files you added, are never deleted.", &mut d.max_items, def.max_items, |ui, max_items| {
        let label = |n: usize| trf("{n} items", &[("n", &n.to_string().as_bytes().rchunks(3).rev().map(|c| std::str::from_utf8(c).unwrap()).collect::<Vec<_>>().join(","))]);
        egui::ComboBox::from_id_salt("history").selected_text(txt(&label(*max_items), 14.0, None)).width(140.0).show_ui(ui, |ui| {
            for n in HISTORY_SIZES {
                ui.selectable_value(max_items, n, txt(&label(n), 14.0, None));
            }
        });
    });
}
