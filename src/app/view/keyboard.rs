//! Keyboard shortcuts and touchpad scrolling.

use super::*;

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

impl App {
    pub(super) fn keyboard(&mut self, ctx: &egui::Context) {
        if self.draft.is_some() {
            // Esc first closes what is open inside Settings: a shortcut field
            // listening (its "cancel"), a menu, or a text field being typed
            // in. Only an Esc with none of these leaves (and applies) Settings.
            let busy = self.recording.is_some()
                || egui::Popup::is_any_open(ctx)
                || ctx.memory(|m| m.focused().is_some());
            if !busy && ctx.input(|i| i.key_pressed(Key::Escape)) {
                self.close_settings(ctx);
            }
            return;
        }
        // The tag field belongs to the item in the details pane: selecting
        // another item or closing the pane closes it. Left open, it would
        // keep every shortcut off below.
        let pane_shown = self.details_open || self.cfg.view == ViewMode::Detail;
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
        let on = |s: &str| !typing && shortcut::pressed(ctx, s);
        let (enter, del, pin, tag, view) = (
            on(&k.copy),
            on(&k.delete),
            on(&k.pin),
            on(&k.tag),
            on(&k.view),
        );
        if view {
            self.set_view(self.cfg.view.next());
        }
        // In the grid ↑ ↓ move a whole row; ← → move one card, unless they
        // would be moving the cursor in a typed query.
        let grid = self.cfg.view == ViewMode::Grid;
        let step = if grid {
            self.grid_cols.max(1) as i32
        } else {
            1
        };
        if down {
            self.move_selection(step);
        }
        if up {
            self.move_selection(-step);
        }
        if grid && self.query.is_empty() {
            // Cards fill from the reading start, so in Arabic the next card is
            // to the left.
            let forward = if i18n::rtl() { -1 } else { 1 };
            if right {
                self.move_selection(forward);
            }
            if left {
                self.move_selection(-forward);
            }
        }
        if let Some(id) = self.selected {
            if enter {
                self.copy(id);
            }
            if del {
                self.delete(id);
            }
            if pin {
                self.toggle_pin(id);
            }
            if tag {
                self.tag_input = Some((id, String::new()));
                self.tag_focus = true;
                self.details_open = true;
            }
        }
        if esc {
            if self.tag_input.is_some() {
                self.tag_input = None;
            } else if self.details_open && self.cfg.view != ViewMode::Detail {
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

    #[test]
    fn mouse_wheel_notches_are_left_alone() {
        let notch = wheel(egui::MouseWheelUnit::Line, Vec2::new(0.0, 1.0));
        assert_eq!(touchpad_scroll(notch.clone(), 40.0), vec![notch]);
        let key = egui::Event::Text("a".into());
        assert_eq!(touchpad_scroll(key.clone(), 40.0), vec![key]);
    }
}
