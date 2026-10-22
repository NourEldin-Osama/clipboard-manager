//! Keyboard shortcuts stored as text ("Ctrl+Shift+P"), the same spelling the
//! global hotkey parser reads, plus the click-and-press recorder that edits them.

use eframe::egui::{Event, KeyboardShortcut, Modifiers, Pos2, Rect, Ui};

use super::icons::{self, Icon};
use super::view::accent;
use super::*;

pub(super) fn parse(s: &str) -> Option<KeyboardShortcut> {
    let mut mods = Modifiers::NONE;
    let mut key = None;
    for part in s.split('+').map(str::trim).filter(|p| !p.is_empty()) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => mods.ctrl = true,
            "alt" | "option" => mods.alt = true,
            "shift" => mods.shift = true,
            "super" | "cmd" | "command" | "meta" | "win" => mods.mac_cmd = true,
            _ => {
                key = egui::Key::from_name(part)
                    .or_else(|| egui::Key::from_name(&part.to_ascii_uppercase()))
            }
        }
    }
    // Ctrl also means ⌘ on macOS, as everywhere else in egui.
    if mods.ctrl && cfg!(target_os = "macos") {
        mods = Modifiers {
            ctrl: false,
            mac_cmd: true,
            command: true,
            ..mods
        };
    } else if mods.ctrl {
        mods.command = true;
    }
    key.map(|k| KeyboardShortcut::new(mods, k))
}

pub(super) fn format(mods: Modifiers, key: egui::Key) -> String {
    let mut parts = Vec::new();
    if mods.ctrl || (mods.command && !mods.mac_cmd) {
        parts.push("Ctrl");
    }
    if mods.alt {
        parts.push("Alt");
    }
    if mods.shift {
        parts.push("Shift");
    }
    if mods.mac_cmd {
        parts.push("Super");
    }
    parts.push(key.name());
    parts.join("+")
}

/// Consumes the shortcut if it was pressed this frame, so text fields don't see it.
pub(super) fn pressed(ctx: &egui::Context, s: &str) -> bool {
    parse(s).is_some_and(|sc| ctx.input_mut(|i| i.consume_shortcut(&sc)))
}

fn is_modifier(k: egui::Key) -> bool {
    let n = k.name();
    ["Control", "Shift", "Alt", "Super", "Meta", "Command"]
        .iter()
        .any(|m| n.starts_with(m))
}

/// Keys that type something: a shortcut on them alone would fire while typing.
fn types_text(k: egui::Key) -> bool {
    let n = k.name();
    n.chars().count() == 1 || matches!(k, egui::Key::Space | egui::Key::Backspace)
}

/// Draws `s` as key caps ("Ctrl" "P"), centred on the row.
pub(super) fn keycaps(ui: &mut Ui, s: &str, dim: bool) {
    let parts: Vec<&str> = s
        .split('+')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for p in parts {
            keycap(ui, p, dim);
        }
    });
}

fn keycap(ui: &mut Ui, label: &str, dim: bool) {
    let v = ui.visuals().clone();
    let color = if dim {
        v.weak_text_color()
    } else {
        v.strong_text_color()
    };
    // Arrow names read better as arrows.
    let shown = match label {
        "Up" | "ArrowUp" => "↑",
        "Down" | "ArrowDown" => "↓",
        "Left" | "ArrowLeft" => "←",
        "Right" | "ArrowRight" => "→",
        l => l,
    };
    let g =
        ui.fonts_mut(|f| f.layout_no_wrap(shown.to_string(), FontId::proportional(12.5), color));
    let size = Vec2::new((g.size().x + 14.0).max(24.0), 24.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let p = ui.painter();
    let fill = if v.dark_mode {
        Color32::from_gray(48)
    } else {
        Color32::WHITE
    };
    let edge = if v.dark_mode {
        Color32::from_gray(70)
    } else {
        Color32::from_gray(210)
    };
    // A slightly darker bottom edge makes it read as a key.
    p.rect_filled(
        rect.translate(Vec2::new(0.0, 1.5)),
        CornerRadius::same(5),
        edge,
    );
    p.rect_filled(rect, CornerRadius::same(5), fill);
    p.rect_stroke(
        rect,
        CornerRadius::same(5),
        Stroke::new(1.0, edge),
        egui::StrokeKind::Inside,
    );
    p.galley(rect.center() - g.size() / 2.0, g, color);
}

const FIELD: Vec2 = Vec2::new(170.0, 32.0);

/// A built-in shortcut, laid out like a recorder so the key caps line up.
pub(super) fn fixed(ui: &mut Ui, s: &str) {
    let (rect, _) = ui.allocate_exact_size(FIELD, Sense::hover());
    let mut inner = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(Vec2::new(10.0, 4.0)))
            .layout(view::row_layout()),
    );
    let keys = if i18n::rtl() {
        s.split('+').rev().collect::<Vec<_>>().join("+")
    } else {
        s.to_string()
    };
    keycaps(&mut inner, &keys, true);
}

/// What the recorder did this frame.
pub(super) enum Recorded {
    Nothing,
    Started,
    Changed,
    Cancelled,
}

/// A shortcut you click and then press. `recording` says whether this field is
/// the one listening; `global` shortcuts need a modifier (or an F key) since
/// they fire in every app.
pub(super) fn recorder(
    ui: &mut Ui,
    value: &mut String,
    recording: bool,
    global: bool,
    problem: &mut Option<&'static str>,
) -> Recorded {
    let mut out = Recorded::Nothing;
    if recording {
        let pressed = ui.input(|i| {
            i.events.iter().find_map(|e| match e {
                // Ctrl, Shift… arrive as keys of their own; wait for the real key.
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } if !is_modifier(*key) => Some((*key, *modifiers)),
                _ => None,
            })
        });
        if let Some((key, mods)) = pressed {
            let bare = !mods.ctrl && !mods.alt && !mods.shift && !mods.mac_cmd && !mods.command;
            if bare && key == egui::Key::Escape {
                out = Recorded::Cancelled;
            } else if bare && matches!(key, egui::Key::Backspace | egui::Key::Delete) {
                value.clear();
                out = Recorded::Changed;
            } else if bare && (types_text(key) || (global && !key.name().starts_with('F'))) {
                *problem = Some(if global {
                    "Add Ctrl, Alt, Shift or Win"
                } else {
                    "Add Ctrl, Alt or Shift"
                });
            } else {
                *value = format(mods, key);
                out = Recorded::Changed;
            }
            // The keys were for the recorder, not for anything else on screen.
            ui.input_mut(|i| {
                i.events
                    .retain(|e| !matches!(e, Event::Key { .. } | Event::Text(_)))
            });
        }
    }
    let (rect, resp) = ui.allocate_exact_size(FIELD, Sense::click());
    let v = ui.visuals().clone();
    let a = accent();
    let bg = if v.dark_mode {
        Color32::from_gray(38)
    } else {
        Color32::from_gray(240)
    };
    let p = ui.painter();
    p.rect_filled(
        rect,
        CornerRadius::same(8),
        if recording {
            a.gamma_multiply(0.10)
        } else {
            bg
        },
    );
    if recording {
        p.rect_stroke(
            rect,
            CornerRadius::same(8),
            Stroke::new(1.5, a),
            egui::StrokeKind::Inside,
        );
    } else if resp.hovered() {
        p.rect_stroke(
            rect,
            CornerRadius::same(8),
            Stroke::new(1.0, v.weak_text_color().gamma_multiply(0.5)),
            egui::StrokeKind::Inside,
        );
    }
    // Content starts at the reading edge; the pen sits at the other end.
    let mut inner = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(Vec2::new(10.0, 4.0)))
            .layout(view::row_layout()),
    );
    if recording {
        let text = problem.unwrap_or("Press keys…");
        inner.label(view::txt(
            tr(text),
            12.5,
            Some(if problem.is_some() { view::RED } else { a }),
        ));
        ui.ctx().request_repaint();
    } else if value.trim().is_empty() {
        inner.label(view::txt(
            tr("Off · click to set"),
            12.5,
            Some(v.weak_text_color()),
        ));
    } else {
        // Key caps always read left to right ("Ctrl" then "P"); a right-to-left row lays them out backwards.
        let keys = if i18n::rtl() {
            value.split('+').rev().collect::<Vec<_>>().join("+")
        } else {
            value.clone()
        };
        keycaps(&mut inner, &keys, false);
        let pen = view::flip(
            rect,
            Rect::from_center_size(
                Pos2::new(rect.right() - 16.0, rect.center().y),
                Vec2::splat(12.0),
            ),
        );
        icons::paint(
            ui.painter(),
            pen,
            Icon::Edit,
            if resp.hovered() {
                v.strong_text_color()
            } else {
                v.weak_text_color()
            },
        );
    }
    let tip = if recording {
        "Press the new keys · Backspace turns it off · Esc cancels"
    } else {
        "Click, then press the new keys"
    };
    let resp = view::tip(resp, tr(tip));
    if resp.clicked() && !recording {
        *problem = None;
        out = Recorded::Started;
    }
    // Clicking anywhere else stops listening.
    if recording
        && matches!(out, Recorded::Nothing)
        && ui.input(|i| i.pointer.any_click())
        && !resp.hovered()
    {
        out = Recorded::Cancelled;
    }
    out
}
