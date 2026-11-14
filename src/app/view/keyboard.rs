//! Keyboard shortcuts and touchpad scrolling.

use super::*;
use egui::text_edit::TextEditState;

/// Splits one fractional wheel-line scroll into point steps that egui applies
/// at once: it eases every line step and point steps of 8 or more over
/// several frames. Windows reports touchpad scrolling as fractions of a line,
/// so with that easing a fast swipe kept the list moving after the fingers
/// stopped. A whole mouse-wheel notch (one line or more) keeps the easing.
pub(super) fn touchpad_scroll(event: egui::Event, line_points: f32) -> Vec<egui::Event> {
    match event {
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta,
            phase,
            source,
            modifiers,
        } if delta != Vec2::ZERO && delta.x.abs() < 1.0 && delta.y.abs() < 1.0 => {
            let points = delta * line_points;
            let steps = (points.length() / 7.0).ceil().max(1.0);
            let step = points / steps;
            (0..steps as usize)
                .map(|_| egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: step,
                    phase,
                    source,
                    modifiers,
                })
                .collect()
        }
        other => vec![other],
    }
}

/// Fixes a frame's input before egui sees it: touchpad scrolling (see
/// `touchpad_scroll`), Shift+Delete (see `shift_delete`) and Ctrl+C (see
/// `ctrl_c`). `modifiers` are the keys held when the frame starts; changes
/// arrive as events.
pub(super) fn repair_input(
    events: Vec<egui::Event>,
    mut modifiers: egui::Modifiers,
    line_points: f32,
) -> Vec<egui::Event> {
    events
        .into_iter()
        .flat_map(|event| {
            if let egui::Event::ModifiersChanged(held) = event {
                modifiers = held;
            }
            let events = ctrl_c(shift_delete(event, modifiers), modifiers);
            events
                .into_iter()
                .flat_map(|event| touchpad_scroll(event, line_points))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// egui-winit turns Ctrl+C into "copy" and drops the key press, so a
/// shortcut of Ctrl+C never arrived. "Copy" while Ctrl is held without
/// Shift or Alt keeps coming, and the frame also gets a press of C: the
/// copy shortcut reads it, and "copy" still copies selected text (see
/// `copies_item`).
fn ctrl_c(event: egui::Event, modifiers: egui::Modifiers) -> Vec<egui::Event> {
    match event {
        egui::Event::Copy if modifiers.command_only() => vec![
            egui::Event::Copy,
            egui::Event::Key {
                key: Key::C,
                physical_key: Some(Key::C),
                pressed: true,
                repeat: false,
                modifiers,
            },
        ],
        other => vec![other],
    }
}

/// Whether a press of the copy shortcut copies the selected item. A press
/// that came with "copy" (Ctrl+C, see `ctrl_c`) doesn't while text is
/// selected in the search field or in a label: "copy" copies that text, as
/// in every program. When it does, "copy" is taken out of the frame, so
/// nothing else copies too.
fn copies_item(ctx: &egui::Context) -> bool {
    if !ctx.input(|i| i.events.contains(&egui::Event::Copy)) {
        return true;
    }
    let in_search = ctx.memory(|m| m.has_focus(search_id()))
        && TextEditState::load(ctx, search_id())
            .and_then(|state| state.cursor.char_range())
            .is_some_and(|range| !range.is_empty());
    let in_label = ctx
        .with_plugin(|labels: &mut egui::text_selection::LabelSelectionState| {
            labels.has_selection()
        })
        .unwrap_or(false);
    if in_search || in_label {
        return false;
    }
    ctx.input_mut(|i| i.events.retain(|e| *e != egui::Event::Copy));
    true
}

/// On Windows, egui-winit turns Shift+Delete into "cut" and drops the key
/// press, so the default Delete shortcut never arrived. Shift+Delete becomes a
/// key press again; Ctrl+X still cuts.
fn shift_delete(event: egui::Event, modifiers: egui::Modifiers) -> egui::Event {
    let shift_only = modifiers.shift && !modifiers.ctrl && !modifiers.command && !modifiers.alt;
    match event {
        egui::Event::Cut if cfg!(windows) && shift_only => egui::Event::Key {
            key: Key::Delete,
            physical_key: Some(Key::Delete),
            pressed: true,
            repeat: false,
            modifiers,
        },
        other => other,
    }
}

/// Which of Ctrl+1 to Ctrl+9 was pressed this frame, counted from 0. The
/// press is used up.
fn quick_paste(ctx: &egui::Context) -> Option<usize> {
    const DIGITS: [Key; 9] = [
        Key::Num1,
        Key::Num2,
        Key::Num3,
        Key::Num4,
        Key::Num5,
        Key::Num6,
        Key::Num7,
        Key::Num8,
        Key::Num9,
    ];
    ctx.input_mut(|i| {
        DIGITS
            .iter()
            .position(|&digit| i.consume_key(egui::Modifiers::COMMAND, digit))
    })
}

impl App {
    /// The keys of the full window, or of the popup when `in_popup`: the same,
    /// less those for what the popup doesn't have (details, tags, views).
    pub(super) fn keyboard(&mut self, ctx: &egui::Context, in_popup: bool) {
        if self.draft.is_some() && !in_popup {
            // Esc first closes what is open inside Settings: a shortcut field
            // listening (its "cancel"), a menu, or a text field being typed
            // in. Only an Esc with none of these goes back: from a server's
            // editor to its page, as the editor's Back button does, and from
            // a page out of Settings, applying them.
            // egui lets go of the text field as soon as Esc arrives, before
            // this runs, so the field typed in at the last frame counts.
            let focused = ctx.memory(|m| m.focused());
            let typing = focused.is_some_and(|id| TextEditState::load(ctx, id).is_some());
            let was_typing = std::mem::replace(&mut self.settings_typing, typing);
            let busy = self.recording.is_some() || egui::Popup::is_any_open(ctx) || was_typing;
            if !busy && ctx.input(|i| i.key_pressed(Key::Escape)) && !self.stash_server_editor() {
                self.close_settings(ctx);
            }
            return;
        }
        if !in_popup {
            self.settings_typing = false;
        }
        // A picture full screen takes the keys.
        if !in_popup && self.viewer.is_some() {
            self.viewer_keys(ctx);
            return;
        }
        // An open item menu takes the keys: the arrows move through it,
        // Enter picks an entry and Esc closes it.
        if self
            .item_menu
            .as_ref()
            .is_some_and(|m| m.in_popup == in_popup)
        {
            return;
        }
        // Any other open menu, such as Sort by or the arrow of the details'
        // main button, takes the keys too: Esc only closes it.
        if egui::Popup::is_any_open(ctx) {
            return;
        }
        // What is new takes the keys while it is open; Esc closes it.
        if self.whats_new_open && !in_popup {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape)) {
                self.whats_new_open = false;
            }
            return;
        }
        // F1 opens the sheet of every key and closes it again; so does Esc.
        let f1 = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::F1));
        if self.keys_sheet == Some(in_popup) {
            let esc = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape));
            if f1 || esc {
                self.keys_sheet = None;
            }
            return;
        }
        if f1 {
            self.keys_sheet = Some(in_popup);
            return;
        }
        // The tag field belongs to the item in the details pane: selecting
        // another item or closing the pane closes it. Left open, it would
        // keep every shortcut off below.
        let pane_shown = !in_popup && (self.details_open || self.cfg.view == ViewMode::Detail);
        if self
            .tag_input
            .as_ref()
            .is_some_and(|(id, _)| Some(*id) != self.selected || !pane_shown)
        {
            self.tag_input = None;
        }
        // Typing in a field other than search (a tag, say) isn't a command.
        // The tag field counts even before egui has moved focus to it.
        let typing = self.tag_input.is_some()
            || self.group_tag.is_some()
            || ctx
                .memory(|m| m.focused())
                .is_some_and(|id| id != search_id());
        let esc = ctx.input(|i| i.key_pressed(Key::Escape));
        let (down, up, left, right) = if typing {
            (false, false, false, false)
        } else {
            ctx.input(|i| {
                (
                    i.key_pressed(Key::ArrowDown),
                    i.key_pressed(Key::ArrowUp),
                    i.key_pressed(Key::ArrowLeft),
                    i.key_pressed(Key::ArrowRight),
                )
            })
        };
        let k = self.cfg.keys.clone();
        let [paste, paste_plain, copy_only, del, pin, tag, view, details] = if typing {
            [false; 8]
        } else {
            shortcut::pressed_each(
                ctx,
                [
                    &k.paste,
                    &k.paste_plain,
                    &k.copy_only,
                    &k.delete,
                    &k.pin,
                    &k.tag,
                    &k.view,
                    &k.details,
                ],
            )
        };
        // Ctrl+C copies the selected item, unless text is selected.
        let copy_only = copy_only && copies_item(ctx);
        // Ctrl+1 to Ctrl+9 paste one of the first nine items shown.
        let quick = if typing { None } else { quick_paste(ctx) };
        // Ctrl+Z undoes the newest delete while it can still be undone, also
        // from the search field (D55); otherwise it stays the field's own
        // undo.
        if !typing
            && self.undo_history.newest().is_some()
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, Key::Z))
        {
            self.run(Command::UndoNewestDelete);
        }
        // Ctrl+A selects every item shown; the popup keeps one at a time.
        if !in_popup
            && !typing
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, Key::A))
        {
            self.mark_all();
        }
        // Shift+F10 opens the selected item's menu, as in other programs.
        let menu = !typing && ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, Key::F10));
        if menu && self.selected.is_some() {
            self.menu_by_key = true;
            self.scroll_to_selected = true;
        }
        // F11 shows the selected picture full screen. A shortcut set to F11
        // in Settings came first and took the press.
        let full_screen = !in_popup
            && !typing
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::F11));
        let copy = if paste {
            Some(Paste::Yes)
        } else if paste_plain {
            Some(Paste::Plain)
        } else if copy_only {
            Some(Paste::No)
        } else {
            None
        };
        if view && !in_popup {
            self.set_view(self.cfg.view.next());
        }
        if copy.is_some() || del || pin || tag || details || full_screen {
            // These act on the selected item: let it follow what was just typed.
            self.finish_search();
        }
        // In the grid ↑ ↓ move a whole row; ← → move one card, unless they
        // would be moving the cursor in a typed query.
        let grid = self.cfg.view == ViewMode::Grid && !in_popup;
        let step = if grid {
            self.grid_cols.max(1) as i32
        } else {
            1
        };
        // Cards as tall as their content stack in columns: ↑ ↓ move within
        // one, ← → to the nearest card in the next one (D69).
        let stacked = grid && !self.grid_places.is_empty();
        if stacked {
            for (pressed, down) in [(down, true), (up, false)] {
                if pressed {
                    self.clear_marks();
                    self.step_in_column(down);
                    self.anchor = self.selected;
                }
            }
        }
        // With Shift, Up and Down take the items passed over into the ones
        // selected together; without, one item is selected again.
        let shift = !in_popup && ctx.input(|i| i.modifiers.shift);
        for (pressed, delta) in [(down, step), (up, -step)] {
            if !pressed || stacked {
                continue;
            }
            let from = self.anchor.or(self.selected);
            if !shift {
                self.clear_marks();
            }
            self.move_selection(delta);
            match (shift, self.selected) {
                (true, Some(to)) => {
                    self.anchor = from;
                    self.mark_range(to);
                }
                _ => self.anchor = self.selected,
            }
        }
        if grid && self.query.is_empty() {
            // Cards fill from the reading start, so in Arabic the next card is
            // to the left.
            let forward = if i18n::rtl() { -1 } else { 1 };
            if right || left {
                self.clear_marks();
            }
            match (stacked, right, left) {
                (true, true, _) => self.step_across(forward),
                (true, _, true) => self.step_across(-forward),
                (false, true, _) => self.move_selection(forward),
                (false, _, true) => self.move_selection(-forward),
                _ => {}
            }
        }
        if let Some(id) = quick.and_then(|n| self.results.get(n)).map(|hit| hit.id) {
            self.run_by_key(Command::Copy(id, Paste::Yes));
        }
        // With several items selected, the keys act on them all.
        let group = if in_popup { Vec::new() } else { self.group() };
        if !group.is_empty() {
            if let Some(paste) = copy {
                self.run_by_key(Command::CopyTogether(group.clone(), paste));
            }
            if del {
                self.run_by_key(Command::DeleteGroup(group.clone()));
            }
            if pin {
                self.run_by_key(Command::PinGroup(group.clone()));
            }
            if tag {
                self.group_tag = Some(String::new());
            }
        } else if let Some(id) = self.selected {
            if let Some(paste) = copy {
                self.run_by_key(Command::Copy(id, paste));
            }
            if del {
                self.run_by_key(Command::Delete(id));
            }
            if pin {
                self.run_by_key(Command::Pin(id));
            }
            if tag && !in_popup {
                self.start_tagging(id);
            }
            // The popup has no details; the key does nothing there.
            if details && !in_popup {
                self.run_by_key(Command::Open(id));
            }
        }
        if let Some(id) = self.selected.filter(|_| full_screen) {
            self.view_full_screen(id);
        }
        if esc {
            // A question waiting for its answer closes first, with no.
            if super::messages::close_question(&mut self.toasts, in_popup) {
                self.focus_search.store(true, Ordering::SeqCst);
            } else if self.group_tag.is_some() {
                self.group_tag = None;
            } else if !in_popup && !self.marked.is_empty() {
                self.clear_marks();
            } else if self.tag_input.is_some() {
                self.tag_input = None;
            } else if !in_popup && self.details_open && self.cfg.view != ViewMode::Detail {
                self.details_open = false;
            } else if self.query.is_empty() {
                self.win.hide();
            } else {
                self.query.clear();
                self.query_vec = None;
                self.dirty = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wheel(unit: egui::MouseWheelUnit, delta: Vec2) -> egui::Event {
        egui::Event::MouseWheel {
            unit,
            delta,
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers: egui::Modifiers::NONE,
        }
    }

    #[test]
    fn touchpad_lines_become_small_immediate_steps() {
        // A fast swipe: 0.9 of a line, 36 points at 40 points per line.
        let out = touchpad_scroll(
            wheel(egui::MouseWheelUnit::Line, Vec2::new(0.0, -0.9)),
            40.0,
        );
        let mut total = Vec2::ZERO;
        for e in &out {
            let egui::Event::MouseWheel { unit, delta, .. } = e else {
                panic!("not a wheel event");
            };
            assert_eq!(*unit, egui::MouseWheelUnit::Point);
            assert!(delta.length() < 8.0, "egui would ease a step of {delta:?}");
            total += *delta;
        }
        assert!((total.y + 36.0).abs() < 1e-3, "{total:?}");
    }

    #[cfg(windows)]
    #[test]
    fn shift_delete_reaches_the_delete_shortcut() {
        // What egui-winit sends for Shift+Delete: Shift goes down, then "cut".
        let shift = egui::Modifiers::SHIFT;
        let events = vec![egui::Event::ModifiersChanged(shift), egui::Event::Cut];
        let input = egui::RawInput {
            events: repair_input(events, egui::Modifiers::NONE, 40.0),
            focused: true,
            ..Default::default()
        };
        let mut pressed = false;
        let mut output = egui::Context::default().run_ui(input, |ui| {
            pressed = shortcut::pressed(ui.ctx(), &crate::config::Keys::default().delete);
        });
        // The font atlas upload no renderer takes here.
        output.textures_delta.clear();
        assert!(pressed, "Shift+Delete must reach the Delete shortcut");
        // Ctrl+X, held since an earlier frame, is still a cut.
        let cut = repair_input(vec![egui::Event::Cut], egui::Modifiers::COMMAND, 40.0);
        assert_eq!(cut, vec![egui::Event::Cut]);
    }

    /// Ctrl held, as egui-winit reports it on Windows.
    const CTRL: egui::Modifiers = egui::Modifiers {
        ctrl: true,
        command: true,
        ..egui::Modifiers::NONE
    };

    /// What egui-winit sends for Ctrl+C: Ctrl goes down, then "copy".
    fn ctrl_c_pressed() -> Vec<egui::Event> {
        let events = vec![egui::Event::ModifiersChanged(CTRL), egui::Event::Copy];
        repair_input(events, egui::Modifiers::NONE, 40.0)
    }

    /// One frame of `events` with the search field holding "hello", its
    /// letters from `selected` selected and the keyboard on it: whether
    /// the copy shortcut `keys` copies the item, whether "copy" is still
    /// in the frame after the keys are read, and what the field copied.
    fn copy_in_search(
        events: Vec<egui::Event>,
        keys: &str,
        selected: std::ops::Range<usize>,
    ) -> (bool, bool, Option<String>) {
        let ctx = egui::Context::default();
        let mut text = String::from("hello");
        let field = |ui: &mut Ui, text: &mut String| {
            ui.add(egui::TextEdit::singleline(text).id(search_id()))
                .request_focus();
        };
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| field(ui, &mut text));
        output.textures_delta.clear();
        let mut state = TextEditState::load(&ctx, search_id()).expect("the field's state");
        let range = egui::text::CCursorRange::two(
            egui::text::CCursor::new(selected.start),
            egui::text::CCursor::new(selected.end),
        );
        state.cursor.set_char_range(Some(range));
        state.store(&ctx, search_id());
        let input = egui::RawInput {
            events,
            focused: true,
            ..Default::default()
        };
        let (mut copies, mut copy_left) = (false, false);
        let mut output = ctx.run_ui(input, |ui| {
            // As `App::keyboard` reads them, before the field is drawn.
            copies = shortcut::pressed(ui.ctx(), keys) && copies_item(ui.ctx());
            copy_left = ui.input(|i| i.events.contains(&egui::Event::Copy));
            field(ui, &mut text);
        });
        output.textures_delta.clear();
        let copied = output
            .platform_output
            .commands
            .iter()
            .find_map(|command| match command {
                egui::OutputCommand::CopyText(text) => Some(text.clone()),
                _ => None,
            });
        (copies, copy_left, copied)
    }

    #[test]
    fn ctrl_c_reaches_the_copy_shortcut() {
        let input = egui::RawInput {
            events: ctrl_c_pressed(),
            focused: true,
            ..Default::default()
        };
        let mut pressed = false;
        let mut output = egui::Context::default().run_ui(input, |ui| {
            pressed = shortcut::pressed(ui.ctx(), &crate::config::Keys::default().copy_only);
        });
        // The font atlas upload no renderer takes here.
        output.textures_delta.clear();
        assert_eq!(crate::config::Keys::default().copy_only, "Ctrl+C");
        assert!(pressed, "Ctrl+C must reach the copy shortcut");
        // Ctrl+Shift+C comes as a key press of its own: no "copy".
        let shift = CTRL | egui::Modifiers::SHIFT;
        let copy = repair_input(vec![egui::Event::Copy], shift, 40.0);
        assert_eq!(copy, vec![egui::Event::Copy]);
    }

    /// With text selected in the search field, Ctrl+C copies that text,
    /// not the item; with none, it copies the item and "copy" is gone, so
    /// nothing else copies too.
    #[test]
    fn ctrl_c_copies_selected_text_else_the_item() {
        let keys = &crate::config::Keys::default().copy_only;
        let (copies, copy_left, copied) = copy_in_search(ctrl_c_pressed(), keys, 0..5);
        assert!(!copies, "the item is not copied");
        assert!(copy_left, "copy stays for the field");
        assert_eq!(copied.as_deref(), Some("hello"));
        let (copies, copy_left, copied) = copy_in_search(ctrl_c_pressed(), keys, 2..2);
        assert!(copies, "the item is copied");
        assert!(!copy_left, "copy is taken out");
        assert_eq!(copied, None);
    }

    /// Settings' recorder takes Ctrl+C, and the keys go to it alone.
    #[test]
    fn the_recorder_takes_ctrl_c() {
        let input = egui::RawInput {
            events: ctrl_c_pressed(),
            focused: true,
            ..Default::default()
        };
        let (mut value, mut problem) = (String::from("Ctrl+Enter"), None);
        let mut left = Vec::new();
        let mut output = egui::Context::default().run_ui(input, |ui| {
            shortcut::recorder(ui, &mut value, true, false, &mut problem);
            left = ui.input(|i| i.events.clone());
        });
        output.textures_delta.clear();
        assert_eq!(value, "Ctrl+C");
        assert_eq!(problem, None);
        let keys_left = left.iter().any(|e| {
            matches!(
                e,
                egui::Event::Key { pressed: true, .. } | egui::Event::Copy
            )
        });
        assert!(!keys_left, "{left:?}");
    }

    /// A copy key saved before Ctrl+C was the default stays, and works.
    #[test]
    fn a_saved_ctrl_enter_still_copies() {
        let saved: crate::config::Keys =
            serde_json::from_str(r#"{"copy_only": "Ctrl+Enter"}"#).expect("saved keys");
        assert_eq!(saved.copy_only, "Ctrl+Enter");
        let enter = vec![egui::Event::Key {
            key: Key::Enter,
            physical_key: Some(Key::Enter),
            pressed: true,
            repeat: false,
            modifiers: CTRL,
        }];
        let (copies, ..) = copy_in_search(enter, &saved.copy_only, 0..5);
        assert!(
            copies,
            "Ctrl+Enter copies the item, even with text selected"
        );
        // Ctrl+C then copies the selected text only.
        let (copies, copy_left, copied) = copy_in_search(ctrl_c_pressed(), &saved.copy_only, 0..5);
        assert!(!copies && copy_left);
        assert_eq!(copied.as_deref(), Some("hello"));
    }

    #[test]
    fn ctrl_and_a_digit_is_a_quick_paste() {
        let press = |key: Key, modifiers: egui::Modifiers| {
            let input = egui::RawInput {
                events: vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                }],
                focused: true,
                ..Default::default()
            };
            let mut found = None;
            let mut output = egui::Context::default().run_ui(input, |ui| {
                found = quick_paste(ui.ctx());
            });
            // The font atlas upload no renderer takes here.
            output.textures_delta.clear();
            found
        };
        let ctrl = egui::Modifiers {
            ctrl: true,
            command: true,
            ..egui::Modifiers::NONE
        };
        assert_eq!(press(Key::Num3, ctrl), Some(2));
        assert_eq!(press(Key::Num9, ctrl), Some(8));
        assert_eq!(press(Key::Num3, egui::Modifiers::NONE), None);
        assert_eq!(press(Key::Num0, ctrl), None);
    }

    #[test]
    fn mouse_wheel_notches_are_left_alone() {
        let notch = wheel(egui::MouseWheelUnit::Line, Vec2::new(0.0, 1.0));
        assert_eq!(touchpad_scroll(notch.clone(), 40.0), vec![notch]);
        let key = egui::Event::Text("a".into());
        assert_eq!(touchpad_scroll(key.clone(), 40.0), vec![key]);
    }
}
