//! Settings, Search: how search works and the meaning-search model.

use super::*;

/// The choices for how close a meaning match must be: a minimum similarity each.
fn strictness() -> [(f32, String); 3] {
    [
        (0.0, t!("settings-strictness-all")),
        (0.15, t!("settings-strictness-related")),
        (0.3, t!("settings-strictness-close")),
    ]
}

impl App {
    pub(super) fn search_page(&mut self, ui: &mut Ui, d: &mut Config, apply_model: &mut bool) {
        let def = Config::default();
        section(ui, &t!("settings-search"));
        setting_for(
            ui,
            &t!("settings-search-mode"),
            &t!("settings-search-mode-help"),
            &mut d.search_mode,
            def.search_mode,
            |ui, mode| {
                segmented(
                    ui,
                    mode,
                    &[
                        (SearchMode::Hybrid, t!("search-mode-smart")),
                        (SearchMode::Semantic, t!("search-mode-meaning")),
                        (SearchMode::Keyword, t!("search-mode-words")),
                    ],
                )
            },
        );
        setting_for(
            ui,
            &t!("settings-strictness"),
            &t!("settings-strictness-help"),
            &mut d.min_similarity,
            def.min_similarity,
            |ui, min_similarity| {
                let choices = strictness();
                let current = choices
                    .iter()
                    .find(|(v, _)| (*v - *min_similarity).abs() < 1e-4)
                    .map(|(_, l)| l.clone())
                    .unwrap_or_else(|| {
                        t!(
                            "settings-strictness-custom",
                            value = format!("{:.2}", min_similarity)
                        )
                    });
                let menu = egui::ComboBox::from_id_salt("strict")
                    .selected_text(txt(&current, 14.0, None))
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        for (v, l) in choices {
                            ui.selectable_value(min_similarity, v, txt(&l, 14.0, None));
                        }
                    });
                set_choice_name(&menu.response, &row_title(ui), &current);
            },
        );

        section(ui, &t!("settings-model-section"));
        let (dot, text) = self.model_status();
        row(ui, |ui| {
            ui.add_space(10.0);
            let color = if dot == Color32::GRAY {
                ui.visuals().weak_text_color()
            } else {
                text_shade(dot, ui.visuals().dark_mode)
            };
            icons::glyph(ui, Icon::Dot, 12.0, color);
            ui.label(txt(&text, 13.0, Some(color)));
        });
        ui.add_space(4.0);
        let mut kind = d.backend.kind();
        plain_setting(
            ui,
            &t!("settings-model"),
            &t!("settings-model-help"),
            |ui| {
                segmented(
                    ui,
                    &mut kind,
                    &[
                        (BackendKind::Off, t!("settings-model-off")),
                        (BackendKind::Server, t!("settings-model-server")),
                    ],
                );
            },
        );
        d.choose_backend(kind);
        match &mut d.backend {
            EmbedBackend::None => note(ui, &t!("settings-model-off-note")),
            EmbedBackend::Server(s) => {
                server_settings(ui, s, &mut d.server_profiles, &mut self.model_list)
            }
        }
        // Applying a server outside this PC starts sending it the whole
        // history, so it is asked about once.
        let send_to = if self.confirm == Some(Confirm::SendToServer) {
            self.unconfirmed_server(d)
        } else {
            None
        };
        if let Some(host) = send_to {
            ui.add_space(8.0);
            Frame::new()
                .inner_margin(Margin::symmetric(10, 0))
                .show(ui, |ui| {
                    let strong = ui.visuals().strong_text_color();
                    let question = t!("settings-server-send-question", host = host.as_str());
                    super::bidi::paragraph(ui, &question, egui::FontId::proportional(13.5), strong);
                });
            ui.add_space(4.0);
            row(ui, |ui| {
                ui.add_space(10.0);
                if primary_button(ui, &t!("settings-server-send")).clicked() {
                    if let EmbedBackend::Server(s) = &d.backend {
                        let url = s.base_url.clone();
                        d.allow_server(&url);
                    }
                    *apply_model = true;
                    self.confirm = None;
                }
                if ui
                    .button(txt(&t!("settings-server-send-cancel"), 13.0, None))
                    .clicked()
                {
                    d.backend = self.cfg.backend.clone();
                    self.confirm = None;
                }
            });
        } else if d.backend != self.cfg.backend {
            if self.confirm == Some(Confirm::SendToServer) {
                self.confirm = None;
            }
            ui.add_space(8.0);
            row(ui, |ui| {
                ui.add_space(10.0);
                if primary_button(ui, &t!("settings-apply-model")).clicked() {
                    *apply_model = true;
                }
                if ui
                    .button(txt(&t!("settings-discard-changes"), 13.0, None))
                    .clicked()
                {
                    d.backend = self.cfg.backend.clone();
                }
                lbl(
                    ui,
                    &t!("settings-model-reindex"),
                    12.5,
                    ui.visuals().weak_text_color(),
                );
            });
        }
    }
}
