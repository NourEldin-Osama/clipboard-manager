//! Keyboard shortcuts stored as text ("Ctrl+Shift+P"), the same spelling the
//! global hotkey parser reads, plus the click-and-press recorder that edits them.

use eframe::egui::{Event, KeyboardShortcut, Modifiers, Pos2, Rect, Ui};

use super::icons::{self, Icon};
use super::view::{accent, radius, size};
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

/// `pressed` for several shortcuts. egui lets an extra Shift or Alt through
/// ("Enter" also takes Shift+Enter), so the shortcuts with more keys are
/// checked first and keep their own presses.
pub(super) fn pressed_each<const N: usize>(ctx: &egui::Context, shortcuts: [&str; N]) -> [bool; N] {
    let mut order: Vec<usize> = (0..N).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(shortcuts[i].split('+').count()));
    let mut each = [false; N];
    for i in order {
        each[i] = pressed(ctx, shortcuts[i]);
    }
    each
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

/// Draws `s` as key caps ("Ctrl" "P"), centred on the row, in their own
/// order in every language (see `Caps`). `dim` draws the keys' names in
/// gray, for keys that can't be changed.
pub(super) fn keycaps(ui: &mut Ui, s: &str, dim: bool) {
    let v = ui.visuals();
    let color = if dim {
        v.weak_text_color()
    } else {
        v.strong_text_color()
    };
    let caps = Caps::new(ui, s, KEYCAPS, (color, v.weak_text_color()));
    let size = Vec2::new(caps.width(), KEYCAPS.height);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    caps.paint(ui.painter(), rect.min, ui.visuals().dark_mode);
}

/// How tall a key cap is.
pub(super) const KEYCAP_HEIGHT: f32 = 24.0;

/// How big a shortcut's key caps are drawn: how tall, and the gap
/// between two.
#[derive(Clone, Copy)]
pub(super) struct CapSize {
    pub(super) height: f32,
    pub(super) gap: f32,
}

/// Settings' and the key sheet's caps.
const KEYCAPS: CapSize = CapSize {
    height: KEYCAP_HEIGHT,
    gap: 4.0,
};

/// A part of a shortcut as drawn: a key on its cap, or a word between two
/// keys, such as "to" in "1 to 9".
enum Piece {
    Cap(Arc<egui::Galley>),
    Word(Arc<egui::Galley>),
}

/// A shortcut laid out as key caps, every place it shows drawing it the
/// same way: in parts that each read left to right (a key combination, a
/// word between keys, a key), in reading order. So in Arabic "Ctrl+1 to
/// 9" reads Ctrl+1 from the right, then the word, then 9.
pub(super) struct Caps {
    /// From the left.
    pieces: Vec<Piece>,
    size: CapSize,
    /// The keys' names, and the words between keys.
    colors: (Color32, Color32),
}

impl Caps {
    /// `keys` ("Ctrl+P", "Ctrl+1 to 9") laid out `size` big, the keys'
    /// names and the words between keys in `colors`.
    pub(super) fn new(ui: &Ui, keys: &str, size: CapSize, colors: (Color32, Color32)) -> Self {
        let cap = |key: &str| Piece::Cap(keycap_label(ui, key, colors.0));
        let (mut units, mut unit) = (Vec::new(), Vec::new());
        for part in keys.split('+').map(str::trim).filter(|k| !k.is_empty()) {
            // A range such as "1 to 9" is two keys with the word between.
            // The first key ends the combination before it, as in Ctrl+1;
            // the word and the last key follow it.
            let words: Vec<&str> = part.split_whitespace().collect();
            if words.len() >= 3 {
                let between = words[1..words.len() - 1].join(" ");
                unit.push(cap(words[0]));
                units.push(std::mem::take(&mut unit));
                units.push(vec![Piece::Word(word_label(ui, &between, colors.1))]);
                unit.push(cap(words[words.len() - 1]));
            } else {
                unit.push(cap(part));
            }
        }
        units.push(unit);
        // From the left: in Arabic the last part first.
        if i18n::rtl() {
            units.reverse();
        }
        Self {
            pieces: units.into_iter().flatten().collect(),
            size,
            colors,
        }
    }

    fn piece_width(&self, piece: &Piece) -> f32 {
        match piece {
            Piece::Cap(g) => keycap_size(g, self.size.height).x,
            Piece::Word(g) => g.size().x,
        }
    }

    /// How wide the caps are, with the gaps between them.
    pub(super) fn width(&self) -> f32 {
        let gaps = self.size.gap * self.pieces.len().saturating_sub(1) as f32;
        self.pieces.iter().map(|p| self.piece_width(p)).sum::<f32>() + gaps
    }

    /// Where the keys' names stand, from the caps' top; the words between
    /// keys, and others beside the caps, stand there too.
    pub(super) fn baseline(&self) -> f32 {
        self.pieces
            .iter()
            .find_map(|piece| match piece {
                Piece::Cap(g) => Some((self.size.height - g.size().y) / 2.0 + baseline(g)),
                Piece::Word(_) => None,
            })
            .unwrap_or(self.size.height / 2.0)
    }

    /// Paints the caps from `at`, their top left corner.
    pub(super) fn paint(&self, p: &egui::Painter, at: Pos2, dark: bool) {
        let base = at.y + self.baseline();
        let mut x = at.x;
        for piece in &self.pieces {
            match piece {
                Piece::Cap(g) => {
                    let size = keycap_size(g, self.size.height);
                    let cap = Rect::from_min_size(Pos2::new(x, at.y), size);
                    paint_keycap(p, cap, g.clone(), self.colors.0, dark);
                }
                Piece::Word(g) => {
                    p.galley(Pos2::new(x, base - baseline(g)), g.clone(), self.colors.1);
                }
            }
            x += self.piece_width(piece) + self.size.gap;
        }
    }
}

/// Where a galley's first line stands, from its top.
pub(super) fn baseline(galley: &egui::Galley) -> f32 {
    galley
        .rows
        .first()
        .and_then(|row| row.row.glyphs.first().map(|glyph| row.pos.y + glyph.pos.y))
        .unwrap_or(galley.size().y)
}

/// Whether the quick paste keys drawn in `shapes` read in order, by where
/// each part's centre is: Ctrl, 1, the word, 9 from the left in English,
/// and from the right in Arabic, where Ctrl+1 still reads from the left,
/// as a key combination is written.
#[cfg(test)]
pub(super) fn quick_paste_order(shapes: &[egui::epaint::ClippedShape]) -> Result<(), String> {
    let keys = t!("quick-paste-keys");
    let word = keys
        .split_whitespace()
        .nth(1)
        .unwrap_or_default()
        .to_string();
    let centres: Vec<(String, f32)> = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) => Some((
                text.galley.text().to_string(),
                text.pos.x + text.galley.rect.center().x,
            )),
            _ => None,
        })
        .collect();
    let at = |piece: &str| {
        centres
            .iter()
            .find(|(text, _)| text == piece)
            .map(|(_, x)| *x)
            .ok_or_else(|| format!("{piece:?} in {centres:?}"))
    };
    let [ctrl, one, word, nine] = [at("Ctrl")?, at("1")?, at(&word)?, at("9")?];
    let from_left = if i18n::rtl() {
        [nine, word, ctrl, one]
    } else {
        [ctrl, one, word, nine]
    };
    if from_left.windows(2).all(|pair| pair[0] < pair[1]) {
        Ok(())
    } else {
        Err(format!("{keys}: {centres:?}"))
    }
}

/// A word between two keys, such as "to" in "1 to 9", in reading order.
fn word_label(ui: &Ui, word: &str, color: Color32) -> Arc<egui::Galley> {
    let font = FontId::proportional(size::SMALL);
    ui.fonts_mut(|f| {
        if super::bidi::has_rtl(word) {
            f.layout_job(super::bidi::job(word, font, color))
        } else {
            f.layout_no_wrap(word.to_string(), font, color)
        }
    })
}

/// A key's name laid out for its cap: the arrow keys as arrows, and a
/// translated name in reading order.
pub(super) fn keycap_label(ui: &Ui, label: &str, color: Color32) -> Arc<egui::Galley> {
    let shown = match label {
        "Up" | "ArrowUp" => "↑",
        "Down" | "ArrowDown" => "↓",
        "Left" | "ArrowLeft" => "←",
        "Right" | "ArrowRight" => "→",
        l => l,
    };
    let font = FontId::proportional(size::SMALL);
    ui.fonts_mut(|f| {
        if super::bidi::has_rtl(shown) {
            f.layout_job(super::bidi::job(shown, font, color))
        } else {
            f.layout_no_wrap(shown.to_string(), font, color)
        }
    })
}

/// The size of a key cap `height` tall around its laid-out name.
pub(super) fn keycap_size(label: &egui::Galley, height: f32) -> Vec2 {
    Vec2::new((label.size().x + 14.0).max(height), height)
}

/// Paints a key cap in `rect`: its face and edge, with a slightly darker
/// bottom edge that makes it read as a key, and its name in the middle.
pub(super) fn paint_keycap(
    p: &egui::Painter,
    rect: Rect,
    label: Arc<egui::Galley>,
    color: Color32,
    dark: bool,
) {
    let (fill, edge) = view::key_cap(dark);
    p.rect_filled(
        rect.translate(Vec2::new(0.0, 1.5)),
        CornerRadius::same(radius::SMALL),
        edge,
    );
    p.rect_filled(rect, CornerRadius::same(radius::SMALL), fill);
    p.rect_stroke(
        rect,
        CornerRadius::same(radius::SMALL),
        Stroke::new(1.0, edge),
        egui::StrokeKind::Inside,
    );
    p.galley(rect.center() - label.size() / 2.0, label, color);
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
    keycaps(&mut inner, s, true);
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
    problem: &mut Option<String>,
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
                    t!("shortcut-needs-modifier-global")
                } else {
                    t!("shortcut-needs-modifier")
                });
            } else {
                *value = format(mods, key);
                out = Recorded::Changed;
            }
            // The keys were for the recorder, not for anything else on
            // screen; Ctrl+C also comes as "copy".
            ui.input_mut(|i| {
                i.events
                    .retain(|e| !matches!(e, Event::Key { .. } | Event::Text(_) | Event::Copy))
            });
        }
    }
    let (rect, resp) = ui.allocate_exact_size(FIELD, Sense::click());
    let v = ui.visuals().clone();
    let a = accent();
    let bg = v.text_edit_bg_color();
    let p = ui.painter();
    p.rect_filled(
        rect,
        CornerRadius::same(radius::MEDIUM),
        if recording {
            a.gamma_multiply(0.10)
        } else {
            bg
        },
    );
    if recording {
        p.rect_stroke(
            rect,
            CornerRadius::same(radius::MEDIUM),
            Stroke::new(1.5, a),
            egui::StrokeKind::Inside,
        );
    } else if resp.hovered() {
        p.rect_stroke(
            rect,
            CornerRadius::same(radius::MEDIUM),
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
        let text = problem.clone().unwrap_or_else(|| t!("shortcut-press-keys"));
        inner.label(view::txt(
            &text,
            size::SMALL,
            Some(view::text_shade(
                if problem.is_some() { view::RED } else { a },
                v.dark_mode,
            )),
        ));
        ui.ctx().request_repaint();
    } else if value.trim().is_empty() {
        inner.label(view::txt(
            &t!("shortcut-off"),
            size::SMALL,
            Some(v.weak_text_color()),
        ));
    } else {
        keycaps(&mut inner, value, false);
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
        t!("shortcut-recording-tip")
    } else {
        t!("shortcut-click-to-set")
    };
    let resp = view::tip(resp, &tip);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Which of `shortcuts` one press of Enter with `modifiers` sets off.
    fn press_enter(modifiers: Modifiers, shortcuts: [&str; 3]) -> [bool; 3] {
        let input = egui::RawInput {
            events: vec![Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }],
            focused: true,
            ..Default::default()
        };
        let mut each = [false; 3];
        let mut output = egui::Context::default().run_ui(input, |ui| {
            each = pressed_each(ui.ctx(), shortcuts);
        });
        // The font atlas upload no renderer takes here.
        output.textures_delta.clear();
        each
    }

    #[test]
    fn a_press_sets_off_only_the_shortcut_with_its_keys() {
        let ctrl = Modifiers {
            ctrl: true,
            command: true,
            ..Modifiers::NONE
        };
        let shortcuts = ["Enter", "Shift+Enter", "Ctrl+Enter"];
        assert_eq!(
            press_enter(Modifiers::NONE, shortcuts),
            [true, false, false]
        );
        assert_eq!(
            press_enter(Modifiers::SHIFT, shortcuts),
            [false, true, false]
        );
        assert_eq!(press_enter(ctrl, shortcuts), [false, false, true]);
        let swapped = ["Shift+Enter", "Enter", "Ctrl+Enter"];
        assert_eq!(press_enter(Modifiers::SHIFT, swapped), [true, false, false]);
    }
}
