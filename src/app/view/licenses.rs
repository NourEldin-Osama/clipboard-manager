//! The licenses of the languages that color code (D95), which ask to be
//! shown with the app: opened from Settings, About.

use super::*;

impl App {
    /// Draws the licenses while they are open over the window.
    pub(super) fn licenses(&mut self, ctx: &egui::Context) {
        if !self.licenses_open {
            return;
        }
        let title = t!("licenses-title");
        let response = whats_new::sheet(ctx, "licenses", &title, &t!("licenses-close"), |ui| {
            // The licenses are in English: they read left to right in either
            // language.
            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                let (strong, weak) = (ui.visuals().strong_text_color(), ui.visuals().text_color());
                for (name, text) in crate::code_colors::licenses() {
                    ui.label(RichText::new(name).size(size::BODY).strong().color(strong));
                    ui.add(
                        egui::Label::new(RichText::new(text).size(size::SMALL).color(weak))
                            .selectable(true)
                            .wrap(),
                    );
                    ui.add_space(space::MEDIUM);
                }
            });
        });
        let (_, clicked) = response.inner;
        if clicked || response.should_close() {
            self.licenses_open = false;
        }
    }
}
