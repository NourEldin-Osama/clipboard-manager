//! Settings, Clipboard: what is kept, and for how long.

use super::*;
use crate::markdown::Marks;

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

/// The Recording section: whether copies are recorded, whether a copy
/// merges with one already in the history, and whether it also merges with
/// one that has other spaces around its text, which counts only while
/// copies merge and is grayed out otherwise.
fn recording_settings(ui: &mut Ui, d: &mut Config) {
    let def = Config::default();
    section(ui, &t!("settings-recording"), |ui| {
        setting_for(
            ui,
            &t!("settings-record"),
            &t!("settings-record-hint"),
            "",
            &mut d.monitor_clipboard,
            def.monitor_clipboard,
            toggle,
        );
        setting_for(
            ui,
            &t!("settings-merge-duplicates"),
            &t!("settings-merge-duplicates-hint"),
            "",
            &mut d.dedupe,
            def.dedupe,
            toggle,
        );
        let merging = d.dedupe;
        let changed = d.dedupe_trimmed != def.dedupe_trimmed;
        // Only the row's content is grayed out: the line above it stays.
        let reset = card_row(ui, changed, |ui| {
            ui.add_enabled_ui(merging, |ui| {
                setting_content(
                    ui,
                    &t!("settings-merge-trimmed"),
                    &t!("settings-merge-trimmed-hint"),
                    "",
                    changed,
                    changed,
                    |ui| {
                        toggle(ui, &mut d.dedupe_trimmed);
                    },
                )
            })
            .inner
        });
        if reset {
            d.dedupe_trimmed = def.dedupe_trimmed;
        }
    });
}

/// The kinds of Markdown marks, in the order Settings lists them: whether
/// each goes, its name, and its marks.
fn mark_kinds(marks: &mut Marks) -> [(&mut bool, String, &'static str); 8] {
    [
        (&mut marks.headings, t!("settings-mark-headings"), "#"),
        (
            &mut marks.bold_and_italic,
            t!("settings-mark-bold-and-italic"),
            "** __ * _",
        ),
        (
            &mut marks.crossed_out,
            t!("settings-mark-crossed-out"),
            "~~",
        ),
        (&mut marks.code, t!("settings-mark-code"), "`"),
        (
            &mut marks.links_and_images,
            t!("settings-mark-links-and-images"),
            "[ ]( )",
        ),
        (
            &mut marks.list_bullets,
            t!("settings-mark-list-bullets"),
            "- * + [ ]",
        ),
        (&mut marks.quotes, t!("settings-mark-quotes"), ">"),
        (
            &mut marks.divider_lines,
            t!("settings-mark-divider-lines"),
            "---",
        ),
    ]
}

/// The Advanced section: how the app copies and pastes items; the history
/// keeps them as they came. While Markdown marks go, a fold row under them
/// lists each kind of mark with a checkbox, so a kind can stay.
fn advanced_settings(ui: &mut Ui, d: &mut Config) {
    let def = Config::default();
    section(ui, &t!("settings-advanced"), |ui| {
        setting_for(
            ui,
            &t!("settings-trim-copied"),
            &t!("settings-trim-copied-hint"),
            "",
            &mut d.trim_copied_text,
            def.trim_copied_text,
            toggle,
        );
        setting_for(
            ui,
            &t!("settings-remove-markdown"),
            &t!("settings-remove-markdown-hint"),
            &t!("settings-remove-markdown-help"),
            &mut d.remove_copied_markdown,
            def.remove_copied_markdown,
            toggle,
        );
        if d.remove_copied_markdown {
            let name = t!("settings-marks-to-remove");
            fold(ui, "markdown-marks", &name, |ui| {
                card_row(ui, false, |ui| {
                    for (goes, label, marks) in mark_kinds(&mut d.markdown_marks) {
                        row(ui, |ui| {
                            checkbox(ui, goes, &label, marks);
                        });
                    }
                });
            });
        }
    });
}

impl App {
    pub(super) fn clipboard_page(&mut self, ui: &mut Ui, d: &mut Config) {
        let def = Config::default();
        recording_settings(ui, d);
        section(ui, &t!("settings-how-long"), |ui| {
            setting_for(
                ui,
                &t!("settings-keep-up-to"),
                &t!("settings-keep-up-to-hint"),
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
                    field_look(ui);
                    let menu = egui::ComboBox::from_id_salt("history")
                        .icon(menu_arrow)
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
                &t!("settings-delete-older-hint"),
                &t!("settings-delete-older-help"),
                &mut d.delete_after_days,
                def.delete_after_days,
                |ui, days| {
                    field_look(ui);
                    let menu = egui::ComboBox::from_id_salt("age")
                        .icon(menu_arrow)
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
        });
        advanced_settings(ui, d);
    }

    fn clear_history_setting(&mut self, ui: &mut Ui) {
        let mut clear = false;
        let confirm = self.confirm;
        let clip = self.library.clearable_count();
        plain_setting(
            ui,
            &t!("settings-clear-history"),
            &t!("settings-clear-history-hint"),
            "",
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
                        .react(ui)
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

#[cfg(test)]
mod tests {
    use super::*;
    use egui::accesskit::{Role, Toggled};

    /// A window in the light theme with the app's fonts and style, which
    /// tells screen readers what it shows.
    fn window() -> egui::Context {
        let ctx = egui::Context::default();
        crate::app::view::add_fonts(&ctx);
        crate::app::view::style(&ctx, DarkPanels::Lighter);
        ctx.set_theme(egui::Theme::Light);
        ctx.enable_accesskit();
        // The fonts take effect from the next frame; the font atlas uploads
        // no renderer takes here are dropped.
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .textures_delta
            .clear();
        ctx
    }

    /// What screen readers hear of one thing: its role, its name, whether
    /// it is on or checked, whether it is open, whether it can't be used,
    /// and its place.
    struct Heard {
        role: Role,
        name: String,
        toggled: Option<Toggled>,
        expanded: Option<bool>,
        disabled: bool,
        place: Rect,
    }

    /// One frame of `draw` with `events`, and what screen readers hear of
    /// it, from the top.
    fn frame(
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        mut draw: impl FnMut(&mut Ui),
    ) -> Vec<Heard> {
        let input = egui::RawInput {
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            // As in the settings' column, where the pages set their gaps.
            ui.spacing_mut().item_spacing.y = 0.0;
            draw(ui);
        });
        output.textures_delta.clear();
        let update = output.platform_output.accesskit_update.expect("a tree");
        let mut heard: Vec<Heard> = update
            .nodes
            .iter()
            .filter_map(|(_, node)| {
                let bounds = node.bounds()?;
                Some(Heard {
                    role: node.role(),
                    name: node.label()?.to_string(),
                    toggled: node.toggled(),
                    expanded: node.is_expanded(),
                    disabled: node.is_disabled(),
                    place: Rect::from_min_max(
                        Pos2::new(bounds.x0 as f32, bounds.y0 as f32),
                        Pos2::new(bounds.x1 as f32, bounds.y1 as f32),
                    ),
                })
            })
            .collect();
        heard.sort_by(|a, b| a.place.top().total_cmp(&b.place.top()));
        heard
    }

    /// The events of a click at `at`, one frame's worth after the other.
    fn click(at: Pos2) -> [Vec<egui::Event>; 2] {
        let press = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        [
            vec![egui::Event::PointerMoved(at), press(true)],
            vec![press(false)],
        ]
    }

    /// The place of the thing with `role` and `name` in `heard`.
    fn find(heard: &[Heard], role: Role, name: &str) -> Rect {
        heard
            .iter()
            .find(|thing| thing.role == role && thing.name == name)
            .unwrap_or_else(|| panic!("{role:?} {name}"))
            .place
    }

    /// The marks to remove show only while Markdown marks go: a closed fold
    /// row, which opens to a checkbox for each kind of mark, in the order
    /// of `mark_kinds`, all checked. Unchecking one keeps that kind.
    #[test]
    fn the_marks_to_remove_show_while_markdown_marks_go() {
        let ctx = window();
        let mut d = Config::default();
        let fold_name = t!("settings-marks-to-remove");
        let heard = frame(&ctx, Vec::new(), |ui| advanced_settings(ui, &mut d));
        assert!(
            !heard.iter().any(|thing| thing.name == fold_name),
            "not while every mark stays"
        );
        d.remove_copied_markdown = true;
        // Each row is laid out by its sizes from the frame before.
        frame(&ctx, Vec::new(), |ui| advanced_settings(ui, &mut d));
        let heard = frame(&ctx, Vec::new(), |ui| advanced_settings(ui, &mut d));
        let fold_row = heard
            .iter()
            .find(|thing| thing.role == Role::Button && thing.name == fold_name)
            .expect("the fold row");
        assert_eq!(fold_row.expanded, Some(false), "closed at first");
        assert!(!heard.iter().any(|thing| thing.role == Role::CheckBox));
        for events in click(fold_row.place.center()) {
            frame(&ctx, events, |ui| advanced_settings(ui, &mut d));
        }
        let heard = frame(&ctx, vec![egui::Event::PointerGone], |ui| {
            advanced_settings(ui, &mut d)
        });
        let boxes: Vec<&Heard> = heard
            .iter()
            .filter(|thing| thing.role == Role::CheckBox)
            .collect();
        let names: Vec<String> = mark_kinds(&mut Marks::default())
            .into_iter()
            .map(|(_, name, _)| name)
            .collect();
        let heard_names: Vec<String> = boxes.iter().map(|thing| thing.name.clone()).collect();
        assert_eq!(heard_names, names);
        assert!(boxes
            .iter()
            .all(|thing| thing.toggled == Some(Toggled::True)));
        let code = find(&heard, Role::CheckBox, &t!("settings-mark-code"));
        for events in click(code.center()) {
            frame(&ctx, events, |ui| advanced_settings(ui, &mut d));
        }
        let want = Marks {
            code: false,
            ..Marks::default()
        };
        assert_eq!(d.markdown_marks, want);
    }

    /// The switch that merges copies with other spaces around their text
    /// is right under Merge duplicate copies, and can't be used while
    /// copies don't merge: screen readers hear that, and a click changes
    /// nothing. Once copies merge, a click turns it on.
    #[test]
    fn merging_around_spaces_waits_for_merging_duplicates() {
        let ctx = window();
        let mut d = Config::default();
        let (merge_name, trimmed_name) = (
            t!("settings-merge-duplicates"),
            t!("settings-merge-trimmed"),
        );
        for merging in [false, true] {
            d.dedupe = merging;
            // Each row is laid out by its sizes from the frame before.
            frame(&ctx, Vec::new(), |ui| recording_settings(ui, &mut d));
            let heard = frame(&ctx, Vec::new(), |ui| recording_settings(ui, &mut d));
            let switches: Vec<&Heard> = heard
                .iter()
                .filter(|thing| thing.role == Role::Switch)
                .collect();
            let names: Vec<&str> = switches.iter().map(|thing| thing.name.as_str()).collect();
            let record = t!("settings-record");
            assert_eq!(names, [&record, &merge_name, &trimmed_name]);
            let (merge, trimmed) = (switches[1], switches[2]);
            assert!(!merge.disabled);
            assert_eq!(trimmed.disabled, !merging, "merging {merging}");
            assert_eq!(trimmed.toggled, Some(Toggled::False));
            for events in click(trimmed.place.center()) {
                frame(&ctx, events, |ui| recording_settings(ui, &mut d));
            }
            assert_eq!(d.dedupe_trimmed, merging, "merging {merging}");
        }
    }
}
