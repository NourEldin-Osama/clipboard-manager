//! Settings, Search & AI: how search works, what searches by meaning, what
//! reads the text in images, and the servers they run on.

use super::*;
use crate::config::{ReadEngine, SearchEngine};

/// The choices for how close a meaning match must be: a minimum similarity each.
fn strictness() -> [(f32, String); 3] {
    [
        (0.0, t!("settings-strictness-all")),
        (0.15, t!("settings-strictness-related")),
        (0.3, t!("settings-strictness-close")),
    ]
}

impl App {
    pub(super) fn search_page(&mut self, ui: &mut Ui, d: &mut Config) {
        if self.server_editor.is_some() {
            self.server_editor_page(ui, d);
            return;
        }
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
        self.send_question(ui, d);

        section(ui, &t!("settings-meaning-section"));
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
        let engine = d.search_engine;
        plain_setting(
            ui,
            &t!("settings-engine"),
            &t!("settings-search-engine-help"),
            |ui| {
                segmented(
                    ui,
                    &mut d.search_engine,
                    &[
                        (SearchEngine::Off, t!("settings-engine-off")),
                        (SearchEngine::Server, t!("settings-engine-server")),
                    ],
                );
            },
        );
        // Turned on with no model picked yet: the first one there is.
        if d.search_engine != engine && !is_model_for(d, d.search_model, true) {
            d.search_model = models_for(d, true).first().copied();
        }
        match d.search_engine {
            SearchEngine::Off => note(ui, &t!("settings-model-off-note")),
            SearchEngine::Server if models_for(d, true).is_empty() => {
                self.no_models(ui, d, &t!("settings-model-none-search"))
            }
            SearchEngine::Server => self.model_menu(ui, d, true),
        }

        section(ui, &t!("settings-read-section"));
        let engine = d.read_engine;
        let mut engines = vec![(ReadEngine::Off, t!("settings-engine-off"))];
        if cfg!(windows) {
            engines.push((ReadEngine::Windows, t!("settings-engine-windows")));
        }
        engines.push((ReadEngine::Server, t!("settings-engine-server")));
        plain_setting(
            ui,
            &t!("settings-engine"),
            &t!("settings-read-engine-help"),
            |ui| {
                segmented(ui, &mut d.read_engine, &engines);
            },
        );
        if d.read_engine != engine && !is_model_for(d, d.read_model, false) {
            d.read_model = models_for(d, false).first().copied();
        }
        match d.read_engine {
            ReadEngine::Off => note(ui, &t!("settings-read-off-note")),
            ReadEngine::Windows => note(ui, &t!("settings-read-windows-note")),
            ReadEngine::Server if models_for(d, false).is_empty() => {
                self.no_models(ui, d, &t!("settings-model-none-read"))
            }
            ReadEngine::Server => self.model_menu(ui, d, false),
        }

        self.servers_section(ui, d);
    }

    /// The engine is a server, but no server has a model for it yet.
    fn no_models(&mut self, ui: &mut Ui, d: &Config, text: &str) {
        note(ui, text);
        row(ui, |ui| {
            ui.add_space(10.0);
            if ui
                .button(txt(&t!("settings-server-add"), 13.0, None))
                .clicked()
            {
                self.add_server(d);
            }
        });
    }
}
