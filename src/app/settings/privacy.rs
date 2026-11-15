//! Settings, Privacy: what is never kept, and where your data goes.

use super::*;

impl App {
    pub(super) fn privacy_page(&mut self, ui: &mut Ui, d: &mut Config) {
        let def = Config::default();
        let mut change = false;
        section(ui, &t!("settings-never-kept"), |ui| {
            setting_for(
                ui,
                &t!("settings-skip-secrets"),
                &t!("settings-skip-secrets-hint"),
                "",
                &mut d.skip_secrets,
                def.skip_secrets,
                toggle,
            );
            setting_for(
                ui,
                &t!("settings-screen-notes"),
                &t!("settings-screen-notes-hint"),
                &t!("settings-screen-notes-help"),
                &mut d.screen_notes,
                def.screen_notes,
                toggle,
            );
            plain_setting(
                ui,
                &t!("settings-skip-next"),
                &t!("settings-skip-next-hint"),
                "",
                |ui| {
                    change = ui
                        .button(txt(&t!("settings-change-key"), size::BODY, None))
                        .clicked();
                    shortcut::fixed(ui, &d.ignore_next_hotkey);
                },
            );
        });
        if change {
            self.settings_page = SettingsPage::Shortcuts;
        }
        self.rules_section(ui, d);
        self.where_data_goes(ui, d);
    }

    /// The programs copies in the history came from, newest first, each
    /// once: its file name, its own name and its icon. Programs a rule
    /// already skips are left out.
    fn history_programs(&mut self, d: &Config) -> Vec<(String, String, Option<TextureHandle>)> {
        let mut items: Vec<_> = self
            .library
            .items()
            .iter()
            .filter(|it| it.app.is_some())
            .cloned()
            .collect();
        items.sort_by_key(|it| std::cmp::Reverse(it.created));
        let mut found: Vec<(String, String, Option<TextureHandle>)> = Vec::new();
        for it in items {
            let Some(app) = it.app.as_deref() else {
                continue;
            };
            let taken = found.iter().any(|(a, _, _)| a.eq_ignore_ascii_case(app));
            if taken || d.rules.iter().any(|r| r.skips_app(app)) {
                continue;
            }
            let program = it.program.as_deref().and_then(|p| self.programs.get(p));
            let name = crate::app::programs::shown_name(app, program);
            let icon = program.and_then(|p| p.icon.clone());
            found.push((app.to_string(), name, icon));
        }
        found
    }

    /// The rules for copies that aren't recorded: one row each, then "Add a
    /// rule", which offers the programs in the history first, with a search
    /// that finds one by its own name or its file name.
    fn rules_section(&mut self, ui: &mut Ui, d: &mut Config) {
        let mut add = None;
        section(ui, &t!("settings-rules"), |ui| {
            note(ui, &t!("settings-rules-help"));
            rule_rows(ui, d);
            card_row(ui, false, |ui| {
                row(ui, |ui| {
                    let button = ui.button(txt(&t!("settings-rule-add"), size::CONTROL, None));
                    let opened = button.clicked();
                    egui::Popup::menu(&button)
                        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                        .show(|ui| {
                            ui.set_width(RULE_MENU_WIDTH);
                            let weak = ui.visuals().weak_text_color();
                            let programs = self.history_programs(d);
                            if !programs.is_empty() {
                                let (field, words) = menu_search_words(
                                    ui,
                                    button.id.with("search"),
                                    opened,
                                    &t!("settings-rule-search"),
                                    RULE_MENU_WIDTH,
                                );
                                let shown: Vec<_> = programs
                                    .iter()
                                    .filter(|(app, name, _)| matches(&words, &[name, app]))
                                    .collect();
                                if enter_in(ui, &field) {
                                    add = shown.first().map(|(app, _, _)| app.clone());
                                }
                                row(ui, |ui| {
                                    ui.add_space(space::SMALL);
                                    lbl(
                                        ui,
                                        &t!("settings-rule-programs").to_uppercase(),
                                        size::SMALL,
                                        weak,
                                    );
                                });
                                let list = button.id.with("list");
                                steady_list(ui, list, opened, RULE_LIST_HEIGHT, |ui| {
                                    if shown.is_empty() {
                                        let none = t!("settings-rule-none-match");
                                        nothing_matches(ui, RULE_MENU_WIDTH, &none);
                                    }
                                    for (app, name, icon) in shown {
                                        let entry = program_row(ui, name, icon.as_ref(), app, None);
                                        if entry.clicked() {
                                            add = Some(app.clone());
                                        }
                                    }
                                });
                                divider(ui);
                            }
                            let other = txt(&t!("settings-rule-add-other"), size::BODY, None);
                            if ui.selectable_label(false, other).clicked() {
                                add = Some(String::new());
                            }
                            if add.is_some() {
                                ui.close();
                            }
                        });
                });
            });
        });
        if let Some(app) = add {
            d.rules.push(crate::rules::Rule {
                app,
                ..Default::default()
            });
        }
    }

    /// Where each kind of data stays or goes, as the settings are now.
    fn where_data_goes(&self, ui: &mut Ui, d: &Config) {
        let dark = ui.visuals().dark_mode;
        let (weak, warn) = (ui.visuals().weak_text_color(), text_shade(AMBER, dark));
        // A server on this PC keeps the data here; any other gets it.
        let server = |url: &str| {
            if crate::net::is_on_this_pc(url) {
                (t!("settings-data-on-pc"), weak)
            } else {
                let host = crate::net::host(url);
                (
                    t!(
                        "settings-data-sent",
                        host = super::bidi::left_to_right(host)
                    ),
                    warn,
                )
            }
        };
        let history = if self.library.store().is_encrypted() {
            t!("settings-data-history-encrypted")
        } else {
            t!("settings-data-history-local")
        };
        let search = match d.search_backend() {
            EmbedBackend::None => (t!("settings-data-off"), weak),
            EmbedBackend::Server(s) => server(&s.base_url),
        };
        let reader = match d.text_reader() {
            TextReader::Off => (t!("settings-data-off"), weak),
            TextReader::Windows => (t!("settings-data-windows"), weak),
            TextReader::Server(r) => server(&r.base_url),
        };
        let updates = if d.update_check {
            t!("settings-data-updates-on")
        } else {
            t!("settings-data-off")
        };
        let rows = [
            (t!("settings-data-history"), (history, weak)),
            (t!("settings-data-search"), search),
            (t!("settings-data-reader"), reader),
            (t!("settings-data-updates"), (updates, weak)),
        ];
        // The answers start on one line, as close to the names as a row lets
        // a control be: a column that ends on the card's far edge, at least
        // as wide as the longest answer.
        let longest = rows
            .iter()
            .map(|(_, (value, color))| {
                txt(value, size::BODY, Some(*color))
                    .into_galley(
                        ui,
                        Some(egui::TextWrapMode::Extend),
                        f32::INFINITY,
                        egui::FontSelection::Default,
                    )
                    .size()
                    .x
            })
            .fold(0.0, f32::max)
            .ceil();
        section(ui, &t!("settings-data-goes"), |ui| {
            for (label, (value, color)) in rows {
                plain_setting(ui, &label, "", "", |ui| {
                    // A point short of the most, so a rounding never puts
                    // the answer under its name.
                    let column = (widest_beside(ui.available_width()) - 1.0).max(longest);
                    let size = Vec2::new(column, ui.spacing().interact_size.y);
                    ui.allocate_ui_with_layout(size, row_layout(), |ui| {
                        ui.set_min_width(column);
                        lbl(ui, &value, size::BODY, color);
                    });
                });
            }
        });
    }
}

/// How wide a rule's two fields are: room for the rest of its row in a
/// settings page.
const RULE_PROGRAM_WIDTH: f32 = 120.0;
const RULE_PATTERN_WIDTH: f32 = 160.0;

/// How wide the menu of "Add a rule" is.
const RULE_MENU_WIDTH: f32 = 340.0;

/// How tall its list of programs grows before it scrolls.
const RULE_LIST_HEIGHT: f32 = 300.0;

/// The rules for copies that aren't recorded, one row each.
fn rule_rows(ui: &mut Ui, d: &mut Config) {
    let mut remove = None;
    for (index, rule) in d.rules.iter_mut().enumerate() {
        ui.push_id(index, |ui| {
            card_row(ui, false, |ui| {
                row(ui, |ui| {
                    let program = ui.add(
                        egui::TextEdit::singleline(&mut rule.app)
                            .hint_text(txt(&t!("settings-rule-program"), size::BODY, None))
                            .desired_width(RULE_PROGRAM_WIDTH),
                    );
                    set_name(&program, &t!("settings-rule-program"));
                    let pattern = ui.add(
                        egui::TextEdit::singleline(&mut rule.pattern)
                            .hint_text(txt(&t!("settings-rule-pattern"), size::BODY, None))
                            .desired_width(RULE_PATTERN_WIDTH),
                    );
                    set_name(&pattern, &t!("settings-rule-pattern"));
                    tip(
                        toggle(ui, &mut rule.is_regex),
                        &t!("settings-rule-regex-help"),
                    );
                    lbl(
                        ui,
                        &t!("settings-rule-regex"),
                        size::CONTROL,
                        ui.visuals().text_color(),
                    );
                    far(ui, |ui| {
                        if icons::icon_button(ui, Icon::Delete, &t!("settings-rule-delete"))
                            .clicked()
                        {
                            remove = Some(index);
                        }
                    });
                });
            });
            match rule.compile() {
                Err(crate::rules::Problem::Empty) => note(ui, &t!("settings-rule-needs")),
                Err(crate::rules::Problem::BadPattern(error)) => {
                    warning(ui, &t!("settings-rule-bad-pattern", error = error.as_str()))
                }
                Ok(_) => {}
            }
        });
    }
    if let Some(index) = remove {
        d.rules.remove(index);
    }
}
