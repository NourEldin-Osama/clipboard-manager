//! Settings, Search & AI: how search works, what searches by meaning and
//! what reads the text in images. The servers they run on have their own
//! page.

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

/// Search by meaning turned on and off, with its state in words under its
/// name, after a dot of the state's color (gray while off).
fn meaning_switch(ui: &mut Ui, engine: &mut SearchEngine, (dot, text): (Color32, String)) {
    card_row(ui, false, |ui| {
        let name = t!("settings-meaning-section");
        let help = t!("settings-search-engine-help");
        let _ = setting_content(ui, &name, "", &help, false, false, |ui| {
            let mut on = *engine == SearchEngine::Server;
            if toggle(ui, &mut on).changed() {
                *engine = if on {
                    SearchEngine::Server
                } else {
                    SearchEngine::Off
                };
            }
        });
        row(ui, |ui| {
            let color = if dot == Color32::GRAY {
                ui.visuals().weak_text_color()
            } else {
                text_shade(dot, ui.visuals().dark_mode)
            };
            icons::glyph(ui, Icon::Dot, 10.0, color);
            ui.label(txt(&text, size::SMALL, Some(color)));
        });
    });
}

impl App {
    pub(super) fn search_page(&mut self, ui: &mut Ui, d: &mut Config) {
        let def = Config::default();
        // The models each saved server has, for the model menus (D67).
        self.server_lists.ask(&d.servers, ui.ctx());
        section(ui, &t!("settings-search"), |ui| {
            setting_for(
                ui,
                &t!("settings-search-mode"),
                // What the chosen way does, in a few words.
                &match d.search_mode {
                    SearchMode::Hybrid => t!("search-mode-smart-help"),
                    SearchMode::Semantic => t!("search-mode-meaning-help"),
                    SearchMode::Keyword => t!("search-mode-words-help"),
                },
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
                &t!("settings-strictness-hint"),
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
                    field_look(ui);
                    let menu = egui::ComboBox::from_id_salt("strict")
                        .icon(menu_arrow)
                        .selected_text(txt(&current, size::BODY, None))
                        .width(MENU_WIDTH)
                        .show_ui(ui, |ui| {
                            for (v, l) in choices {
                                ui.selectable_value(min_similarity, v, txt(&l, size::BODY, None));
                            }
                        });
                    set_choice_name(&menu.response, &row_title(ui), &current);
                },
            );
            self.send_question(ui, d);
        });

        let state = self.meaning_state();
        // Without a search model in use, the ones found on this PC come first.
        let in_use =
            d.search_engine == SearchEngine::Server && is_model_for(d, d.search_model, true);
        self.local_servers.look(ui.ctx());
        section(ui, &t!("settings-meaning-section"), |ui| {
            let engine = d.search_engine;
            meaning_switch(ui, &mut d.search_engine, state);
            // Turned on with no model picked yet: the first one there is.
            if d.search_engine != engine && !is_model_for(d, d.search_model, true) {
                d.search_model = models_for(d, true).first().copied();
            }
            if !in_use {
                self.found_rows(ui, d);
            }
            match d.search_engine {
                SearchEngine::Off => note(ui, &t!("settings-model-off-note")),
                SearchEngine::Server if models_for(d, true).is_empty() => {
                    self.no_models(ui, d, &t!("settings-model-none-search"))
                }
                SearchEngine::Server => self.model_menu(ui, d, true),
            }
        });

        section(ui, &t!("settings-read-section"), |ui| {
            let engine = d.read_engine;
            let mut engines = vec![(ReadEngine::Off, t!("settings-engine-off"))];
            if cfg!(windows) {
                engines.push((ReadEngine::Windows, t!("settings-engine-windows")));
            }
            engines.push((ReadEngine::Server, t!("settings-engine-server")));
            plain_setting(
                ui,
                &t!("settings-read-engine"),
                &t!("settings-read-engine-hint"),
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
        });
    }

    /// The engine is a server, but no server has a model for it yet.
    fn no_models(&mut self, ui: &mut Ui, d: &Config, text: &str) {
        note(ui, text);
        card_row(ui, false, |ui| {
            row(ui, |ui| {
                if ui
                    .button(txt(&t!("settings-server-add"), size::CONTROL, None))
                    .clicked()
                {
                    self.add_server(d);
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One frame of the switch with `events`, in a window with the app's
    /// fonts and style: what was painted, and the screen reader's tree.
    fn switch_frame(
        ctx: &egui::Context,
        engine: &mut SearchEngine,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let input = egui::RawInput {
            events,
            ..Default::default()
        };
        let state = (Color32::GRAY, t!("settings-meaning-off"));
        let mut output = ctx.run_ui(input, |ui| {
            let column = Rect::from_min_size(Pos2::new(20.0, 20.0), Vec2::new(600.0, 300.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(column), |ui| {
                card(ui, |ui| meaning_switch(ui, engine, state.clone()));
            });
        });
        // The font atlas upload no renderer takes here.
        output.textures_delta.clear();
        output
    }

    /// The row's switch carries the section's name for screen readers and
    /// turns search by meaning on; its state stands under its name.
    #[test]
    fn search_by_meaning_is_a_switch_with_its_state_under_its_name() {
        let ctx = egui::Context::default();
        crate::app::view::add_fonts(&ctx);
        crate::app::view::style(&ctx, DarkPanels::Lighter);
        ctx.enable_accesskit();
        let mut engine = SearchEngine::Off;
        // The fonts apply from the next frame.
        switch_frame(&ctx, &mut engine, Vec::new());
        let output = switch_frame(&ctx, &mut engine, Vec::new());
        let name = t!("settings-meaning-section");
        let state = t!("settings-meaning-off");
        let text_rect = |words: &str| {
            output
                .shapes
                .iter()
                .find_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) if text.galley.text() == words => {
                        Some(text.galley.rect.translate(text.pos.to_vec2()))
                    }
                    _ => None,
                })
                .expect(words)
        };
        let (title, line) = (text_rect(&name), text_rect(&state));
        assert!(line.top() >= title.bottom(), "{line:?} under {title:?}");
        // The dot at the name's reading-start edge, the words after it.
        let dot = output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Circle(circle) if line.y_range().contains(circle.center.y) => {
                    Some(circle.center)
                }
                _ => None,
            })
            .expect("a dot");
        assert!(
            (dot.x - 5.0 - title.left()).abs() <= 1.0,
            "{dot:?} {title:?}"
        );
        assert!(line.left() > dot.x, "{line:?} after {dot:?}");
        let update = output.platform_output.accesskit_update.expect("a tree");
        let switch = update
            .nodes
            .iter()
            .map(|(_, node)| node)
            .find(|node| node.role() == egui::accesskit::Role::Switch)
            .expect("a switch");
        assert_eq!(switch.label(), Some(name.as_str()));
        let bounds = switch.bounds().expect("a place");
        let at = Pos2::new(
            ((bounds.x0 + bounds.x1) / 2.0) as f32,
            ((bounds.y0 + bounds.y1) / 2.0) as f32,
        );
        let press = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        switch_frame(
            &ctx,
            &mut engine,
            vec![egui::Event::PointerMoved(at), press(true)],
        );
        switch_frame(&ctx, &mut engine, vec![press(false)]);
        assert_eq!(engine, SearchEngine::Server);
    }
}
