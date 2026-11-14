//! Keyboard shortcuts and touchpad scrolling.

use super::*;
use egui::text_edit::TextEditState;

/// A wheel step in lines, whether it is in fractions of a line, and the
/// keys held. Windows reports a mouse wheel in whole notches (a line each)
/// and a touchpad in fractions of one, also when a fast swipe adds up to
/// several lines. Elsewhere only steps under a line count as fractions.
fn wheel_step(event: &egui::Event) -> Option<(Vec2, bool, egui::Modifiers)> {
    let egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta,
        modifiers,
        ..
    } = event
    else {
        return None;
    };
    // Windows' own unit: 120 to a notch.
    let whole = |lines: f32| ((lines * 120.0).round() as i32) % 120 == 0;
    let fraction = if cfg!(windows) {
        !(whole(delta.x) && whole(delta.y))
    } else {
        delta.x.abs() < 1.0 && delta.y.abs() < 1.0
    };
    (*delta != Vec2::ZERO).then_some((*delta, fraction, *modifiers))
}

/// A movement of `points` as wheel steps that egui applies at once: it
/// eases every line step and point steps of 8 or more over several frames.
fn point_steps(points: Vec2, modifiers: egui::Modifiers) -> Vec<egui::Event> {
    let steps = (points.length() / 7.0).ceil().max(1.0);
    let step = points / steps;
    (0..steps as usize)
        .map(|_| egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: step,
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers,
        })
        .collect()
}

/// Fingers moving on a touchpad send a step every 8 to 17 ms, 40 ms apart
/// at most as a swipe starts; Windows sends a swipe's coasting 50 to 70 ms
/// apart.
const FINGER_GAP: f64 = 0.045;

/// A step this soon after the first of a run shows fingers moving: a
/// swipe's second step comes 6 to 9 ms after its first.
const FINGER_FOLLOW: f64 = 0.02;

/// Otherwise this many steps in a row, each at most `FINGER_GAP` after the
/// one before, show fingers moving.
const FINGER_RUN: usize = 3;

/// A run that starts with a step up to this many lines starts as soon as a
/// second step comes (`FINGER_FOLLOW`). A bigger first step needs the whole
/// run: a fast swipe's steps keep coming, while Windows' coasting held back
/// as the window drew comes as one big step and at most a small one after.
const QUICK_START_MOST: f32 = 15.0;

/// A step in whole lines this soon after a touchpad's step is the
/// touchpad's too: now and then a step adds up to whole lines.
const TOUCHPAD_FOR: f64 = 1.0;

/// How closely the list follows the fingers: it shows the part of their
/// steps it hasn't shown yet in about this many seconds, so uneven steps
/// make an even movement.
const FOLLOW: f32 = 0.03;

/// After the fingers lift the list coasts on, its speed falling 2.7 times
/// every this many seconds; in all it goes its speed times this far.
const COAST: f32 = 0.3;

/// The fastest the list coasts, in points a second.
const COAST_TOP_SPEED: f32 = 15_000.0;

/// A list coasting faster than the top speed comes down to it over about
/// this many seconds, rather than at once.
const TOP_SPEED_EASE: f32 = 0.1;

/// Fingers lifting slower than this, in points a second, leave the list
/// where they left it: only a quick swipe coasts.
const COAST_START_SPEED: f32 = 1_500.0;

/// Below this speed, in points a second, the list stops.
const STOP_SPEED: f32 = 40.0;

/// The longest time a frame counts as, in seconds: the first frame after
/// a rest moves as one frame, not by the whole rest.
const LONGEST_FRAME: f32 = 0.02;

/// Touchpad scrolling as egui gets it, smooth and coasting like a browser.
///
/// Windows' own coasting after a swipe comes only while the window is
/// idle: while it draws, the coasting waits and then comes as one big step,
/// often tens of lines, as the drawing stops. The list stopped where the
/// fingers left it, then jumped on. So only fingers scroll: a touchpad step
/// counts once it is part of a run of steps close together, and a run too
/// short (a lone step, a big step with a small one after it) is Windows'
/// coasting, left out. The list then coasts on its own from the speed the
/// fingers left it at.
#[derive(Default)]
pub(crate) struct Touchpad {
    /// When the last touchpad step came, in egui's time.
    last: Option<f64>,
    /// A run of steps, in points, not yet long enough to show fingers
    /// moving, and when it began.
    waiting: Vec<Vec2>,
    waiting_since: f64,
    /// The points the fingers moved that the list hasn't shown yet.
    owed: Vec2,
    /// How fast the list moved in the last frame, in points a second.
    speed: Vec2,
    /// Whether the list is coasting: it keeps its pace between the
    /// fingers' steps and after they lift.
    coasting: bool,
    /// When the last frame was.
    frame: Option<f64>,
}

impl Touchpad {
    /// This frame's `events` at egui's time `now`, with the touchpad's
    /// steps (`line_points` to a line) turned into the list's smooth
    /// movement, and Windows' coasting left out; and whether the list
    /// moves on in the next frame.
    pub(crate) fn steps(
        &mut self,
        events: Vec<egui::Event>,
        now: f64,
        line_points: f32,
    ) -> (Vec<egui::Event>, bool) {
        let dt = self.frame.map_or(0.0, |frame| {
            ((now - frame) as f32).clamp(0.0, LONGEST_FRAME)
        });
        self.frame = Some(now);
        let close = |last: Option<f64>| last.is_some_and(|last| now - last <= FINGER_GAP);
        // Nothing followed the waiting steps soon enough: they were coasting.
        if !close(self.last) {
            self.waiting.clear();
        }
        let mut out = Vec::new();
        let mut fingers = None;
        for event in events {
            if matches!(event, egui::Event::PointerButton { pressed: true, .. }) {
                // A click stops the list.
                (self.owed, self.speed, self.coasting) = (Vec2::ZERO, Vec2::ZERO, false);
            }
            let Some((step, fraction, keys)) = wheel_step(&event) else {
                out.push(event);
                continue;
            };
            let touchpad = fraction
                || (cfg!(windows) && self.last.is_some_and(|last| now - last <= TOUCHPAD_FOR));
            if !touchpad {
                out.push(event);
                continue;
            }
            if !cfg!(windows) || !keys.is_none() {
                // Elsewhere, and with a key held (Ctrl for a pinch), at once.
                out.extend(point_steps(step * line_points, keys));
                continue;
            }
            let points = step * line_points;
            let moving = close(self.last) && self.waiting.is_empty();
            self.last = Some(now);
            if moving {
                *fingers.get_or_insert(Vec2::ZERO) += points;
                continue;
            }
            if self.waiting.is_empty() {
                self.waiting_since = now;
            }
            let quick = self.waiting.first().is_some_and(|first| {
                first.length() <= QUICK_START_MOST * line_points
                    && now - self.waiting_since <= FINGER_FOLLOW
            });
            self.waiting.push(points);
            if quick || self.waiting.len() >= FINGER_RUN {
                // Fingers: the run scrolls, but not a coasting step that
                // went the other way.
                for waiting in self.waiting.drain(..) {
                    if waiting.dot(points) > 0.0 {
                        *fingers.get_or_insert(Vec2::ZERO) += waiting;
                    }
                }
            }
        }
        let lifted = !close(self.last);
        let moved = self.motion(fingers, dt, lifted);
        if moved != Vec2::ZERO {
            out.extend(point_steps(moved, egui::Modifiers::NONE));
        }
        (out, self.speed != Vec2::ZERO || self.owed != Vec2::ZERO)
    }

    /// How far the list moves in a frame `dt` seconds after the last one,
    /// with the fingers' new steps, if any; `lifted` once no step has come
    /// for a while.
    fn motion(&mut self, fingers: Option<Vec2>, dt: f32, lifted: bool) -> Vec2 {
        let pace = self.speed * (-dt / COAST).exp();
        if let Some(fingers) = fingers {
            // Fingers the other way stop the list first.
            if fingers.dot(self.speed) < 0.0 {
                (self.owed, self.speed) = (Vec2::ZERO, Vec2::ZERO);
            }
            self.owed += fingers;
            self.coasting = false;
        } else if self.speed.length() >= COAST_START_SPEED {
            self.coasting = true;
        }
        let catch_up = self.owed * (1.0 - (-dt / FOLLOW).exp());
        // Between steps and after the fingers lift, a quick list keeps its
        // pace, slowing down as it coasts; new steps lead it.
        let keep_pace =
            self.coasting && pace.length() * dt > catch_up.length() && pace.dot(self.owed) >= 0.0;
        let moved = if keep_pace { pace * dt } else { catch_up };
        // What it shows past the fingers' steps is coasting.
        let left = self.owed - moved;
        self.owed = if left.dot(self.owed) > 0.0 {
            left
        } else {
            Vec2::ZERO
        };
        if dt > 0.0 {
            self.speed = moved / dt;
        }
        // Only once the fingers lift: between a swipe's steps the list keeps
        // up with them, however fast.
        let over = self.speed.length() - COAST_TOP_SPEED;
        if lifted && over > 0.0 {
            let eased = COAST_TOP_SPEED + over * (-dt / TOP_SPEED_EASE).exp();
            self.speed = self.speed.normalized() * eased;
        }
        if self.speed.length() < STOP_SPEED && self.owed.length() < 0.5 {
            (self.owed, self.speed, self.coasting) = (Vec2::ZERO, Vec2::ZERO, false);
        }
        moved
    }
}

/// Fixes a frame's keys before egui sees them: Shift+Delete (see
/// `shift_delete`) and Ctrl+C (see `ctrl_c`). `modifiers` are the keys held
/// when the frame starts; changes arrive as events.
pub(super) fn repair_input(
    events: Vec<egui::Event>,
    mut modifiers: egui::Modifiers,
) -> Vec<egui::Event> {
    events
        .into_iter()
        .flat_map(|event| {
            if let egui::Event::ModifiersChanged(held) = event {
                modifiers = held;
            }
            ctrl_c(shift_delete(event, modifiers), modifiers)
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

    /// Points the events scroll by at 40 points to a line, checking that
    /// a touchpad's come as steps egui applies at once.
    fn scrolled(events: &[egui::Event]) -> f32 {
        let mut total = 0.0;
        for e in events {
            let egui::Event::MouseWheel { unit, delta, .. } = e else {
                continue;
            };
            match unit {
                egui::MouseWheelUnit::Point => {
                    assert!(delta.length() < 8.0, "egui would ease a step of {delta:?}");
                    total += delta.y;
                }
                _ => total += delta.y * 40.0,
            }
        }
        total
    }

    /// One frame at `now` with a wheel step of `lines`, if any: the points
    /// the list moves by.
    fn frame(touchpad: &mut Touchpad, now: f64, lines: Option<f32>) -> f32 {
        let events = lines
            .map(|lines| wheel(egui::MouseWheelUnit::Line, Vec2::new(0.0, lines)))
            .into_iter()
            .collect();
        scrolled(&touchpad.steps(events, now, 40.0).0)
    }

    /// Frames 1/60 s apart from `from` to `to`, and the recorded `steps`
    /// at their own times: when each frame was and how far it moved the
    /// list.
    fn replay(
        touchpad: &mut Touchpad,
        from: f64,
        to: f64,
        steps: &[(f64, f32)],
    ) -> Vec<(f64, f32)> {
        let mut frames: Vec<(f64, Option<f32>)> =
            steps.iter().map(|&(t, l)| (t, Some(l))).collect();
        let mut t = from;
        while t < to {
            frames.push((t, None));
            t += 1.0 / 60.0;
        }
        frames.sort_by(|a, b| a.0.total_cmp(&b.0));
        frames
            .into_iter()
            .map(|(t, l)| (t, frame(touchpad, t, l)))
            .collect()
    }

    fn total(frames: &[(f64, f32)]) -> f32 {
        frames.iter().map(|(_, moved)| moved).sum()
    }

    /// A swipe of `steps` lines each, one per frame from `from`.
    fn swipe(from: f64, steps: &[f32]) -> Vec<(f64, f32)> {
        let frame = 1.0 / 60.0;
        steps
            .iter()
            .enumerate()
            .map(|(n, &lines)| (from + frame * n as f64 + 0.001, lines))
            .collect()
    }

    #[test]
    fn a_touchpad_moves_the_list_in_steps_egui_shows_at_once() {
        // A slow swipe of 0.9 of a line: 36 points at 40 points per line,
        // and the list stops with the fingers.
        let frames = replay(&mut Touchpad::default(), 0.0, 1.0, &swipe(0.0, &[-0.3; 3]));
        let moved = total(&frames);
        assert!((moved + 36.0).abs() < 0.6, "{moved}");
    }

    /// Recorded on a Windows touchpad: swipes down, each followed by
    /// Windows' coasting, either as big steps up to half a second later
    /// (while the window was drawing) or as steps 50 to 70 ms apart, one
    /// of them in whole lines. After each swipe the list only slows down
    /// until the next one: it never stops and then jumps on.
    #[cfg(windows)]
    #[test]
    fn windows_coasting_never_makes_the_list_jump() {
        let swipes: [&[(f64, f32)]; 3] = [
            &[
                (0.0, -2.79),
                (0.0402, -2.00),
                (0.0567, -2.23),
                (0.0735, -3.47),
                (0.0904, -2.85),
                (0.1073, -5.27),
                (0.1239, -8.38),
            ],
            &[
                (2.1151, -2.17),
                (2.1392, -0.56),
                (2.1555, -1.32),
                (2.1721, -1.73),
                (2.1891, -3.65),
                (2.2054, -6.62),
            ],
            &[
                (5.181, -3.91),
                (5.190, -1.58),
                (5.225, -3.67),
                (5.242, -4.06),
                (5.258, -3.91),
            ],
        ];
        let coasting = [
            (0.4418, -39.75),
            (0.8069, -0.97),
            (2.5059, -20.90),
            (5.309, -17.08),
            (5.375, -14.88),
            (5.443, -13.11),
            (5.509, -11.00),
            (5.808, -5.08),
            (5.836, -0.06),
        ];
        let all: Vec<(f64, f32)> = swipes.concat().into_iter().chain(coasting).collect();
        let frames = replay(&mut Touchpad::default(), 0.0, 8.0, &all);
        let starts: Vec<f64> = swipes.iter().map(|s| s[0].0).chain([8.0]).collect();
        // How fast the list went in each frame, in points a second, as the
        // touchpad counts a frame's time.
        let speeds: Vec<(f64, f32)> = frames
            .windows(2)
            .map(|pair| {
                let dt = ((pair[1].0 - pair[0].0) as f32).clamp(1e-4, LONGEST_FRAME);
                (pair[1].0, pair[1].1 / dt)
            })
            .collect();
        for (swipe, next) in swipes.iter().zip(&starts[1..]) {
            let lifted = swipe.last().expect("a step").0;
            let after: Vec<f32> = speeds
                .iter()
                .filter(|(t, _)| *t > lifted && t < next)
                .map(|(_, speed)| *speed)
                .collect();
            for pair in after.windows(2) {
                assert!(
                    pair[1] <= 0.0 && pair[1] >= pair[0] * 1.02 - 1.0,
                    "the list sped up after the swipe ending at {lifted} s: {after:?}"
                );
            }
            assert_eq!(after.last(), Some(&0.0), "it stops before the next swipe");
        }
    }

    /// The list coasts on at the fingers' speed and slows down: in all a
    /// little less than its speed times `COAST` beyond the fingers.
    #[cfg(windows)]
    #[test]
    fn the_list_coasts_on_from_the_fingers_speed() {
        // 2.05 lines a frame: 82 points, 4,920 points a second.
        let steps = swipe(0.0, &[-2.05; 18]);
        let lifted = steps.last().expect("a step").0;
        let frames = replay(&mut Touchpad::default(), 0.0, 4.0, &steps);
        let first_after = frames.iter().find(|(t, _)| *t > lifted).expect("a frame").1;
        assert!(first_after < -60.0, "it keeps its pace: {first_after}");
        let coast = total(&frames) + 18.0 * 82.0;
        assert!(coast < -1200.0 && coast > -1700.0, "{coast}");
    }

    /// A swipe's second step soon after its first starts the list at once;
    /// a step alone, Windows' late coasting, never moves it.
    #[cfg(windows)]
    #[test]
    fn a_swipe_starts_at_once_and_a_lone_step_waits() {
        let mut touchpad = Touchpad::default();
        let lone = replay(&mut touchpad, 0.0, 1.0, &[(0.0, -20.3)]);
        assert_eq!(total(&lone), 0.0);
        let started = replay(&mut touchpad, 1.0, 1.05, &[(1.001, -1.05), (1.009, -1.05)]);
        assert!(total(&started) < -50.0, "{started:?}");
    }

    /// Recorded: as the list stopped coasting, Windows let out an earlier
    /// swipe's coasting as one step of 207 lines, and a small step 16 ms
    /// later. Neither moves the list.
    #[cfg(windows)]
    #[test]
    fn a_late_big_step_with_a_small_one_after_it_never_moves_the_list() {
        let mut touchpad = Touchpad::default();
        replay(&mut touchpad, 0.0, 2.0, &swipe(0.0, &[-2.05; 18]));
        let late = replay(&mut touchpad, 2.0, 3.0, &[(2.001, 207.24), (2.0177, 13.27)]);
        assert_eq!(total(&late), 0.0, "{late:?}");
    }

    /// Recorded: a fast swipe whose steps are tens of lines from the first
    /// one. All of it moves the list.
    #[cfg(windows)]
    #[test]
    fn a_fast_swipe_with_big_first_steps_moves_the_list() {
        let mut touchpad = Touchpad::default();
        let lines = [-25.93, -38.22, -25.90];
        let moved: f32 = swipe(0.0, &lines)
            .into_iter()
            .map(|(t, l)| frame(&mut touchpad, t, Some(l)))
            .sum();
        // What it has shown and what it is still to show: every step.
        let counted = moved + touchpad.owed.y;
        let fingers: f32 = lines.iter().map(|l| l * 40.0).sum();
        assert!((counted - fingers).abs() < 1.0, "{counted} of {fingers}");
    }

    /// A list going faster than the top speed when the fingers lift comes
    /// down to it over a moment: it isn't cut at once.
    #[cfg(windows)]
    #[test]
    fn a_very_fast_swipe_slows_down_gently() {
        // 30.5 lines a frame: 73,200 points a second.
        let steps = swipe(0.0, &[-30.5; 12]);
        let lifted = steps.last().expect("a step").0;
        let frames = replay(&mut Touchpad::default(), 0.0, 1.0, &steps);
        let after: Vec<f32> = frames
            .iter()
            .filter(|(t, _)| *t > lifted + FINGER_GAP)
            .take(3)
            .map(|(_, moved)| -moved * 60.0)
            .collect();
        assert!(after[0] > 2.0 * COAST_TOP_SPEED, "cut at once: {after:?}");
        assert!(after.windows(2).all(|pair| pair[1] < pair[0]), "{after:?}");
    }

    /// Fingers the other way, or a click, stop the coasting list.
    #[cfg(windows)]
    #[test]
    fn fingers_the_other_way_or_a_click_stop_the_list() {
        let mut touchpad = Touchpad::default();
        replay(&mut touchpad, 0.0, 0.4, &swipe(0.0, &[-2.05; 18]));
        let back = replay(&mut touchpad, 0.4, 0.5, &swipe(0.401, &[1.05; 4]));
        let last = back.last().expect("a frame").1;
        assert!(last > 0.0, "it goes the fingers' way: {back:?}");
        replay(&mut touchpad, 0.5, 1.0, &swipe(0.6, &[-2.05; 18]));
        let click = egui::Event::PointerButton {
            pos: Pos2::ZERO,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        };
        let (events, moving) = touchpad.steps(vec![click], 1.0, 40.0);
        assert_eq!((scrolled(&events), moving), (0.0, false));
    }

    #[cfg(windows)]
    #[test]
    fn shift_delete_reaches_the_delete_shortcut() {
        // What egui-winit sends for Shift+Delete: Shift goes down, then "cut".
        let shift = egui::Modifiers::SHIFT;
        let events = vec![egui::Event::ModifiersChanged(shift), egui::Event::Cut];
        let input = egui::RawInput {
            events: repair_input(events, egui::Modifiers::NONE),
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
        let cut = repair_input(vec![egui::Event::Cut], egui::Modifiers::COMMAND);
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
        repair_input(events, egui::Modifiers::NONE)
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
        let copy = repair_input(vec![egui::Event::Copy], shift);
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

    /// Whole notches keep egui's easing, also after a pause, and other
    /// events pass as they are.
    #[test]
    fn mouse_wheel_notches_are_left_alone() {
        let mut touchpad = Touchpad::default();
        let notch = wheel(egui::MouseWheelUnit::Line, Vec2::new(0.0, 1.0));
        let key = egui::Event::Text("a".into());
        for now in [0.0, 0.3, 0.6] {
            let events = vec![notch.clone(), key.clone()];
            let (out, moving) = touchpad.steps(events.clone(), now, 40.0);
            assert_eq!((out, moving), (events, false));
        }
    }
}
