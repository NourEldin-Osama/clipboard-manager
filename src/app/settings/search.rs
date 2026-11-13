//! Settings, AI & Search: what searches by meaning, how search works and
//! what reads the text in images. The servers they run on have their own
//! page.

use super::*;
use crate::config::{ReadEngine, SearchEngine};

/// The choices for how close a meaning match must be: a minimum similarity each.
fn strictness() -> [(f32, String); 3] {
    [
        (0.0, t!("settings-strictness-any")),
        (0.15, t!("settings-strictness-close")),
        (0.3, t!("settings-strictness-very-close")),
    ]
}

/// Search by meaning turned on and off: its state as a chip after its name,
/// in the state's color (gray while off), and what the state means in a
/// gray line under the name. Screen readers hear both with the switch.
fn meaning_switch(
    ui: &mut Ui,
    engine: &mut SearchEngine,
    (color, words, line): (Color32, String, String),
) {
    card_row(ui, false, |ui| {
        let name = t!("settings-meaning-switch");
        let help = t!("settings-search-engine-help");
        let spoken = t!(
            "settings-meaning-spoken",
            state = words.as_str(),
            line = line.as_str()
        );
        let state = StateChip { color, words };
        let weak = ui.visuals().weak_text_color();
        let under = |ui: &mut Ui| {
            super::bidi::balanced(ui, &line, FontId::proportional(size::SMALL), weak);
        };
        let _ = setting_parts(
            ui,
            &name,
            Some(&state),
            Some(under),
            &help,
            false,
            false,
            |ui| {
                let mut on = *engine == SearchEngine::Server;
                let switch = toggle(ui, &mut on);
                ui.ctx()
                    .accesskit_node_builder(switch.id, |node| node.set_description(spoken));
                if switch.changed() {
                    *engine = if on {
                        SearchEngine::Server
                    } else {
                        SearchEngine::Off
                    };
                }
            },
        );
    });
}

impl App {
    pub(super) fn search_page(&mut self, ui: &mut Ui, d: &mut Config) {
        let def = Config::default();
        // The models each saved server has, for the model menus (D67).
        self.server_lists.ask(&d.servers, ui.ctx());
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
            // While it is on: how close its matches must be, then the model
            // in use under Advanced, as most people keep the model they set
            // up once. A model still to choose comes first, as its menu did.
            let model_in_use =
                d.search_engine == SearchEngine::Server && is_model_for(d, d.search_model, true);
            if d.search_engine == SearchEngine::Off {
                note(ui, &t!("settings-model-off-note"));
            } else {
                if models_for(d, true).is_empty() {
                    self.no_models(ui, d, &t!("settings-model-none-search"));
                } else if !model_in_use {
                    self.model_menu(ui, d, true);
                }
                setting_for(
                    ui,
                    &t!("settings-strictness"),
                    &t!("settings-strictness-hint"),
                    "",
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
                                    ui.selectable_value(
                                        min_similarity,
                                        v,
                                        txt(&l, size::BODY, None),
                                    );
                                }
                            });
                        set_choice_name(&menu.response, &row_title(ui), &current);
                    },
                );
                if model_in_use {
                    advanced(ui, "search-model", |ui| self.model_menu(ui, d, true));
                }
            }
            self.send_question(ui, d);
        });

        section(ui, &t!("settings-search"), |ui| {
            // What the chosen way does, in a few words; without search by
            // meaning, every way finds the words only.
            let line = match (d.search_engine, d.search_mode) {
                (SearchEngine::Off, _) => t!("settings-search-mode-meaning-off"),
                (_, SearchMode::Hybrid) => t!("search-mode-smart-help"),
                (_, SearchMode::Semantic) => t!("search-mode-meaning-help"),
                (_, SearchMode::Keyword) => t!("search-mode-words-help"),
            };
            setting_for(
                ui,
                &t!("settings-search-mode"),
                &line,
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
    use crate::i18n::arabic_run;

    /// What one frame of the switch's row shows: the card's inside, the
    /// name's lines, the chip and its words and dot, the gray line's lines,
    /// the info button, and the switch with what screen readers hear of it.
    struct Row {
        inside: Rect,
        name: Vec<Rect>,
        chip: Rect,
        chip_corners: CornerRadius,
        words: Rect,
        dot: Pos2,
        line: Vec<Rect>,
        info: Rect,
        switch: Rect,
        switch_label: Option<String>,
        switch_description: Option<String>,
        chip_spoken_as: Option<egui::accesskit::Role>,
    }

    /// One frame of the switch with `events`, in `state`, in a column `width`
    /// wide in a window with the app's fonts and style.
    fn switch_frame(
        ctx: &egui::Context,
        engine: &mut SearchEngine,
        state: &(Color32, String, String),
        width: f32,
        events: Vec<egui::Event>,
    ) -> Row {
        let input = egui::RawInput {
            events,
            ..Default::default()
        };
        let (mut inside, mut fill) = (Rect::NOTHING, Color32::TRANSPARENT);
        let mut output = ctx.run_ui(input, |ui| {
            let column = Rect::from_min_size(Pos2::new(20.0, 20.0), Vec2::new(width, 300.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(column), |ui| {
                card(ui, |ui| {
                    meaning_switch(ui, engine, state.clone());
                    inside = ui.min_rect();
                });
            });
            // The chip's tint, of the rows' gray for a state without a color.
            let (color, ..) = state;
            let tint = if *color == Color32::GRAY {
                ui.visuals().weak_text_color()
            } else {
                *color
            };
            fill = tint.gamma_multiply(0.14);
        });
        // The font atlas upload no renderer takes here.
        output.textures_delta.clear();
        let mut texts = Vec::new();
        let (mut chip, mut chip_corners) = (Rect::NOTHING, CornerRadius::ZERO);
        let mut dots = Vec::new();
        for clipped in &output.shapes {
            match &clipped.shape {
                egui::Shape::Text(text) => {
                    let size = text.galley.job.sections[0].format.font_id.size;
                    texts.push((size, text.galley.rect.translate(text.pos.to_vec2())));
                }
                egui::Shape::Rect(rect) if rect.fill == fill => {
                    chip = rect.rect;
                    chip_corners = rect.corner_radius;
                }
                egui::Shape::Circle(circle) => dots.push(circle.center),
                _ => {}
            }
        }
        let name: Vec<Rect> = texts
            .iter()
            .filter(|(size, _)| *size == size::BODY)
            .map(|(_, at)| *at)
            .collect();
        let words = texts
            .iter()
            .find(|(_, at)| chip.contains(at.center()))
            .map(|(_, at)| *at)
            .expect("the chip's words");
        let line: Vec<Rect> = texts
            .iter()
            .filter(|(size, at)| *size == size::SMALL && !chip.contains(at.center()))
            .map(|(_, at)| *at)
            .collect();
        let dot = dots
            .into_iter()
            .find(|dot| chip.contains(*dot))
            .expect("a dot in the chip");
        let update = output.platform_output.accesskit_update.expect("a tree");
        let place = |node: &egui::accesskit::Node| {
            let bounds = node.bounds().expect("a place");
            Rect::from_min_max(
                Pos2::new(bounds.x0 as f32, bounds.y0 as f32),
                Pos2::new(bounds.x1 as f32, bounds.y1 as f32),
            )
        };
        let nodes: Vec<&egui::accesskit::Node> =
            update.nodes.iter().map(|(_, node)| node).collect();
        let switch = nodes
            .iter()
            .find(|node| node.role() == egui::accesskit::Role::Switch)
            .expect("a switch");
        let help = t!("settings-search-engine-help");
        let info = nodes
            .iter()
            .find(|node| node.label() == Some(help.as_str()))
            .expect("the info button");
        // A label's text is its value.
        let chip_spoken_as = nodes
            .iter()
            .find(|node| node.value() == Some(state.1.as_str()))
            .map(|node| node.role());
        Row {
            inside,
            name,
            chip,
            chip_corners,
            words,
            dot,
            line,
            info: place(info),
            switch: place(switch),
            switch_label: switch.label().map(str::to_string),
            switch_description: switch.description().map(str::to_string),
            chip_spoken_as,
        }
    }

    /// A window with the app's fonts and style, which tells screen readers
    /// what it shows.
    fn window() -> egui::Context {
        let ctx = egui::Context::default();
        crate::app::view::add_fonts(&ctx);
        crate::app::view::style(&ctx, DarkPanels::Lighter);
        ctx.enable_accesskit();
        ctx
    }

    /// The reading-start edge of `rect`.
    fn start_edge(rect: Rect) -> f32 {
        if i18n::rtl() {
            rect.right()
        } else {
            rect.left()
        }
    }

    /// The far edge of `rect`.
    fn far_edge(rect: Rect) -> f32 {
        if i18n::rtl() {
            rect.left()
        } else {
            rect.right()
        }
    }

    /// How far `a` comes before `b` in reading order.
    fn before(a: f32, b: f32) -> f32 {
        if i18n::rtl() {
            a - b
        } else {
            b - a
        }
    }

    /// Where the row's controls end: its far edge, past the rows' room at
    /// their sides.
    fn controls_end(row: &Row) -> f32 {
        far_edge(row.inside.shrink2(Vec2::new(space::LARGE, 0.0)))
    }

    /// The switch's row with search by meaning off: the chip sits on the
    /// name's line after the name and its info button, as tall as a tag's
    /// chip with its corners, its dot first; the gray line starts under the
    /// name where a gray line starts; the switch is at the row's far end,
    /// carries the section's name for screen readers with the state said
    /// with it, and turns search by meaning on.
    fn check_switch_and_state() {
        let ctx = window();
        let mut engine = SearchEngine::Off;
        let state = (
            Color32::GRAY,
            t!("settings-meaning-chip-off"),
            t!("settings-meaning-off"),
        );
        // The fonts apply from the next frame.
        switch_frame(&ctx, &mut engine, &state, 600.0, Vec::new());
        let row = switch_frame(&ctx, &mut engine, &state, 600.0, Vec::new());
        assert_eq!(row.name.len(), 1, "the name on one line: {:?}", row.name);
        let title = row.name[0];
        // On the name's line, after the name and the info button.
        let off = row.chip.center().y - title.center().y;
        assert!(off.abs() <= 0.5, "{:?} on {title:?}", row.chip);
        assert!(before(far_edge(title), start_edge(row.info)) >= 0.0);
        assert!(before(far_edge(row.info), start_edge(row.chip)) >= 0.0);
        let height = row.words.height() + 2.0 * CARD_CHIP_PADDING;
        assert!((row.chip.height() - height).abs() <= 0.5, "{:?}", row.chip);
        assert_eq!(row.chip_corners, CornerRadius::same(radius::MEDIUM));
        let dot_first = before(row.dot.x, start_edge(row.words));
        assert!(dot_first > 0.0, "{:?} before {:?}", row.dot, row.words);
        // Where a gray line starts.
        let line = row.line[0];
        let gap = line.top() - title.bottom();
        assert!((gap - space::TINY).abs() <= 0.5, "{line:?} under {title:?}");
        let along = start_edge(line) - start_edge(title);
        assert!(along.abs() <= 0.5, "{line:?} under {title:?}");
        let end = far_edge(row.switch) - controls_end(&row);
        assert!(end.abs() <= 0.5, "{:?} in {:?}", row.switch, row.inside);
        // Centred on the name and the gray line together.
        let text = row.line.iter().fold(title, |text, line| text.union(*line));
        let off = row.switch.center().y - text.center().y;
        assert!(off.abs() <= 0.5, "{:?} on {text:?}", row.switch);
        let name = t!("settings-meaning-switch");
        assert_eq!(row.switch_label.as_deref(), Some(name.as_str()));
        let spoken = t!(
            "settings-meaning-spoken",
            state = state.1.as_str(),
            line = state.2.as_str()
        );
        assert_eq!(row.switch_description, Some(spoken));
        assert_eq!(row.chip_spoken_as, Some(egui::accesskit::Role::Label));
        let at = row.switch.center();
        let press = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let events = vec![egui::Event::PointerMoved(at), press(true)];
        switch_frame(&ctx, &mut engine, &state, 600.0, events);
        switch_frame(&ctx, &mut engine, &state, 600.0, vec![press(false)]);
        assert_eq!(engine, SearchEngine::Server);
    }

    #[test]
    fn search_by_meaning_is_a_switch_with_its_state_under_its_name() {
        check_switch_and_state();
    }

    #[test]
    fn search_by_meaning_is_a_switch_with_its_state_under_its_name_right_to_left() {
        crate::i18n::passes_in_arabic(
            module_path!(),
            "search_by_meaning_is_a_switch_with_its_state_under_its_name_in_arabic",
        );
    }

    #[test]
    #[ignore = "run in a process of its own by search_by_meaning_is_a_switch_with_its_state_under_its_name_right_to_left"]
    fn search_by_meaning_is_a_switch_with_its_state_under_its_name_in_arabic() {
        if arabic_run() {
            check_switch_and_state();
        }
    }

    /// The switch's row in each state in a column just wide enough for the
    /// switch beside its text: the switch stays there, at the row's far
    /// end, nothing comes nearer to it than a setting's text does, and the
    /// name wraps where its line with the info button and the chip would
    /// not fit.
    fn check_narrow_row() {
        let ctx = window();
        let gap = ctx.global_style().spacing.item_spacing.x;
        let states = [
            (
                GREEN,
                "settings-meaning-chip-ready",
                "settings-meaning-ready",
            ),
            (
                Color32::GRAY,
                "settings-meaning-chip-off",
                "settings-meaning-off",
            ),
            (
                AMBER,
                "settings-meaning-chip-loading",
                "settings-meaning-loading",
            ),
            (
                Color32::GRAY,
                "settings-meaning-chip-not-set-up",
                "settings-meaning-none",
            ),
            (
                RED,
                "settings-meaning-chip-not-working",
                "settings-meaning-failing",
            ),
        ];
        for (color, words, line) in states {
            let state = (
                color,
                crate::i18n::text(words, None),
                crate::i18n::text(line, None),
            );
            let mut engine = SearchEngine::Server;
            // Laid out by the sizes of the frame before.
            let mut draw = |width: f32| {
                let mut row = switch_frame(&ctx, &mut engine, &state, width, Vec::new());
                for _ in 0..3 {
                    row = switch_frame(&ctx, &mut engine, &state, width, Vec::new());
                }
                row
            };
            let wide = draw(600.0);
            let one_line = wide.name[0].width() + gap + wide.info.width() + gap + wide.chip.width();
            let row = draw(360.0);
            let end = far_edge(row.switch) - controls_end(&row);
            assert!(
                end.abs() <= 0.5,
                "{words}: {:?} in {:?}",
                row.switch,
                row.inside
            );
            let text = row
                .name
                .iter()
                .chain(&row.line)
                .fold(row.chip, |text, part| text.union(*part));
            let middle = row.switch.center().y;
            assert!(
                text.y_range().contains(middle),
                "{words}: {:?} beside {text:?}",
                row.switch
            );
            let room = before(far_edge(text), start_edge(row.switch));
            assert!(
                room >= space::HUGE - 0.5,
                "{words}: {room}: {text:?} before {:?}",
                row.switch
            );
            let start = start_edge(row.inside.shrink2(Vec2::new(space::LARGE, 0.0)));
            let text_room = before(start, start_edge(row.switch)) - space::HUGE;
            let wraps = one_line > text_room;
            assert_eq!(
                row.name.len() > 1,
                wraps,
                "{words}: {one_line} in {text_room}"
            );
        }
    }

    #[test]
    fn a_narrow_row_wraps_the_name_before_the_switch_moves() {
        check_narrow_row();
    }

    #[test]
    fn a_narrow_row_wraps_the_name_before_the_switch_moves_right_to_left() {
        crate::i18n::passes_in_arabic(
            module_path!(),
            "a_narrow_row_wraps_the_name_before_the_switch_moves_in_arabic",
        );
    }

    #[test]
    #[ignore = "run in a process of its own by a_narrow_row_wraps_the_name_before_the_switch_moves_right_to_left"]
    fn a_narrow_row_wraps_the_name_before_the_switch_moves_in_arabic() {
        if arabic_run() {
            check_narrow_row();
        }
    }
}
