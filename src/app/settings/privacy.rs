//! Settings, Privacy: what is never kept, where your data goes, and backups.

use super::*;

/// How long the line of a running backup's progress is.
const BACKUP_PROGRESS_WIDTH: f32 = 120.0;

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
        self.backup_section(ui);
    }

    /// Writing the history to a file locked with a password, and restoring
    /// one; while one runs, how far it is.
    fn backup_section(&mut self, ui: &mut Ui) {
        let stage = self.backup_stage();
        let (mut export, mut import) = (false, false);
        section(ui, &t!("settings-backup"), |ui| {
            plain_setting(
                ui,
                &t!("settings-backup-file"),
                &t!("settings-backup-file-hint"),
                "",
                |ui| {
                    // From the far end; one backup at a time.
                    let idle = stage.is_none();
                    let button = |label: &str| egui::Button::new(txt(label, size::BODY, None));
                    import = ui
                        .add_enabled(idle, button(&t!("settings-backup-import")))
                        .clicked();
                    export = ui
                        .add_enabled(idle, button(&t!("settings-backup-export")))
                        .clicked();
                },
            );
            let Some(stage) = stage else {
                return;
            };
            card_row(ui, false, |ui| {
                row(ui, |ui| {
                    let weak = ui.visuals().weak_text_color();
                    slow_spinner(ui, 14.0, Some(weak));
                    let (text, part) = match stage {
                        BackupStage::Saving { done, total } => (
                            t!("settings-backup-saving", done = done, total = total),
                            Some((done, total)),
                        ),
                        BackupStage::Checking => (t!("settings-backup-checking"), None),
                        BackupStage::Restoring { done, total } => (
                            t!("settings-backup-restoring", done = done, total = total),
                            Some((done, total)),
                        ),
                    };
                    lbl(ui, &text, size::SMALL, weak);
                    if let Some((done, total)) = part {
                        let (bar, _) = ui.allocate_exact_size(
                            Vec2::new(BACKUP_PROGRESS_WIDTH, PROGRESS_LINE),
                            Sense::hover(),
                        );
                        let done = done as f32 / total.max(1) as f32;
                        paint_progress(ui.painter(), bar, done, ui.visuals().dark_mode);
                    }
                });
            });
        });
        if export {
            self.ask_backup_password();
        }
        if import {
            self.run(Command::ImportBackup);
        }
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
        section(ui, &t!("settings-data-goes"), |ui| data_rows(ui, &rows));
    }
}

/// The room between the longest name of "Where your data goes" and the
/// answers.
const DATA_ANSWER_GAP: f32 = 2.0 * space::LARGE;

/// One row for each name and its answer in its color: the names at the
/// reading start, and the answers on their lines, all starting
/// `DATA_ANSWER_GAP` after the longest name. Where the longest answer
/// doesn't fit there, each answer goes under its name.
fn data_rows(ui: &mut Ui, rows: &[(String, (String, Color32))]) {
    let strong = ui.visuals().strong_text_color();
    let width = |ui: &Ui, text: &str, color: Color32| {
        txt(text, size::BODY, Some(color))
            .into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                egui::FontSelection::Default,
            )
            .size()
            .x
    };
    let longest_name = rows
        .iter()
        .map(|(name, _)| width(ui, name, strong))
        .fold(0.0, f32::max)
        .ceil();
    let longest_answer = rows
        .iter()
        .map(|(_, (answer, color))| width(ui, answer, *color))
        .fold(0.0, f32::max)
        .ceil();
    let answers_at = longest_name + DATA_ANSWER_GAP;
    for (name, (answer, color)) in rows {
        card_row(ui, false, |ui| {
            if answers_at + longest_answer <= ui.available_width() {
                row(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let shown = lbl(ui, name, size::BODY, strong);
                    ui.add_space(answers_at - shown.rect.width());
                    lbl(ui, answer, size::BODY, *color);
                });
            } else {
                row(ui, |ui| lbl(ui, name, size::BODY, strong));
                let font = FontId::proportional(size::BODY);
                super::bidi::paragraph(ui, answer, font, *color);
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
                        Field::singleline(&mut rule.app)
                            .hint_text(txt(&t!("settings-rule-program"), size::BODY, None))
                            .desired_width(RULE_PROGRAM_WIDTH),
                    );
                    set_name(&program, &t!("settings-rule-program"));
                    let pattern = ui.add(
                        Field::singleline(&mut rule.pattern)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::arabic_run;

    /// The reading-start edge of `rect`.
    fn start_edge(rect: Rect) -> f32 {
        if i18n::rtl() {
            rect.right()
        } else {
            rect.left()
        }
    }

    /// The far edge of `rect`, where its text ends.
    fn far_edge(rect: Rect) -> f32 {
        if i18n::rtl() {
            rect.left()
        } else {
            rect.right()
        }
    }

    /// How far `x` is from `from` in reading order.
    fn along(from: f32, x: f32) -> f32 {
        if i18n::rtl() {
            from - x
        } else {
            x - from
        }
    }

    /// A line of text: the place it was laid out in, and where its letters
    /// are drawn. egui's place for a line of Arabic set against its right
    /// edge leaves out the room before its first letter, so the letters
    /// reach past it.
    #[derive(Debug)]
    struct Line {
        place: Rect,
        letters: Rect,
    }

    /// One frame of "Where your data goes" with its longest answers, in a
    /// card in a column `width` wide in a window with the app's fonts and
    /// style: where the card's rows start, and the names' and the answers'
    /// lines, from the top.
    fn data_frame(ctx: &egui::Context, width: f32) -> (f32, Vec<Line>, Vec<Line>) {
        let v = ctx.style_of(egui::Theme::Light).visuals.clone();
        let (strong, weak) = (v.strong_text_color(), v.weak_text_color());
        let host = super::super::bidi::left_to_right("models.example.com");
        let rows = [
            (
                t!("settings-data-history"),
                (t!("settings-data-history-encrypted"), weak),
            ),
            (
                t!("settings-data-search"),
                (
                    t!("settings-data-sent", host = host),
                    text_shade(AMBER, false),
                ),
            ),
            (
                t!("settings-data-reader"),
                (t!("settings-data-windows"), weak),
            ),
            (
                t!("settings-data-updates"),
                (t!("settings-data-updates-on"), weak),
            ),
        ];
        let mut inside = Rect::NOTHING;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            // As in the settings' column, where the pages set their gaps.
            ui.spacing_mut().item_spacing.y = 0.0;
            let column = Rect::from_min_size(Pos2::new(20.0, 20.0), Vec2::new(width, 600.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(column), |ui| {
                card(ui, |ui| {
                    data_rows(ui, &rows);
                    inside = ui.min_rect();
                });
            });
        });
        output.textures_delta.clear();
        let (mut names, mut answers) = (Vec::new(), Vec::new());
        for clipped in &output.shapes {
            if let egui::Shape::Text(text) = &clipped.shape {
                let line = Line {
                    place: text.galley.rect.translate(text.pos.to_vec2()),
                    letters: text.galley.mesh_bounds.translate(text.pos.to_vec2()),
                };
                if text.galley.job.sections[0].format.color == strong {
                    names.push(line);
                } else {
                    answers.push(line);
                }
            }
        }
        for lines in [&mut names, &mut answers] {
            lines.sort_by(|a, b| a.place.top().total_cmp(&b.place.top()));
        }
        // The rows' room at their sides.
        let rows_start = start_edge(inside.shrink2(Vec2::new(space::LARGE, 0.0)));
        (rows_start, names, answers)
    }

    /// A window with the app's fonts and style in the light theme.
    fn window() -> egui::Context {
        let ctx = egui::Context::default();
        crate::app::view::add_fonts(&ctx);
        crate::app::view::style(&ctx, DarkPanels::Lighter);
        ctx.set_theme(egui::Theme::Light);
        // The fonts take effect from the next frame; the font atlas uploads
        // no renderer takes here are dropped.
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .textures_delta
            .clear();
        ctx
    }

    /// In the widest column each name starts at its row's reading start,
    /// and each answer on its name's line, all of them at one place: two
    /// large gaps after the letters of the longest name, give or take the
    /// room beside a letter and the longest name's width rounded up to a
    /// whole point. In a narrow column each answer goes under its name,
    /// from the row's reading start.
    fn check_answers_after_the_longest_name() {
        let ctx = window();
        let (start, names, answers) = data_frame(&ctx, super::super::COLUMN_WIDTH);
        assert_eq!((names.len(), answers.len()), (4, 4), "{answers:?}");
        let longest = names
            .iter()
            .map(|name| along(start, far_edge(name.letters)))
            .fold(0.0, f32::max);
        let first_answer = along(start, start_edge(answers[0].place));
        for (name, answer) in names.iter().zip(&answers) {
            let name_off = along(start, start_edge(name.place));
            assert!(name_off.abs() <= 0.5, "{name:?} from {start}");
            let answer_off = along(start, start_edge(answer.place)) - first_answer;
            assert!(answer_off.abs() <= 0.5, "{answer:?} with the others");
            let gap = along(start, start_edge(answer.letters)) - longest;
            assert!(
                (DATA_ANSWER_GAP - 0.5..=DATA_ANSWER_GAP + 2.5).contains(&gap),
                "{answer:?}: {gap} after the longest name"
            );
            let off = answer.place.center().y - name.place.center().y;
            assert!(off.abs() <= 0.5, "{answer:?} on the line of {name:?}");
        }

        let (start, names, answers) = data_frame(&ctx, 300.0);
        assert_eq!(names.len(), 4);
        let mut answer_lines = answers.iter();
        for (index, name) in names.iter().enumerate() {
            let first = answer_lines
                .find(|line| line.place.top() >= name.place.bottom())
                .expect("an answer under the name");
            let next_name = names
                .get(index + 1)
                .map_or(f32::INFINITY, |next| next.place.top());
            assert!(
                first.place.bottom() <= next_name,
                "{first:?} under {name:?}"
            );
            let off = along(start, start_edge(first.place));
            assert!(off.abs() <= 0.5, "{first:?} from {start}");
        }
    }

    #[test]
    fn the_answers_start_after_the_longest_name() {
        check_answers_after_the_longest_name();
    }

    #[test]
    fn the_answers_start_after_the_longest_name_right_to_left() {
        crate::i18n::passes_in_arabic(
            module_path!(),
            "the_answers_start_after_the_longest_name_in_arabic",
        );
    }

    #[test]
    #[ignore = "run in a process of its own by the_answers_start_after_the_longest_name_right_to_left"]
    fn the_answers_start_after_the_longest_name_in_arabic() {
        if arabic_run() {
            check_answers_after_the_longest_name();
        }
    }
}
