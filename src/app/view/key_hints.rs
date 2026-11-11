//! Quiet lines of keys and what they do: the popup's foot and the full
//! window's status bar. The keys show as key caps, as Settings shows them,
//! and the words stand on the line the caps' names stand on.

use super::*;

/// How tall the hints' key caps are: a little smaller than Settings', so
/// two lines of them leave room for the popup's list.
pub(super) const HINT_CAP: f32 = 22.0;

/// Between the caps of one shortcut.
const CAP_GAP: f32 = 3.0;

/// Between the keys and their words.
const WORDS_GAP: f32 = 6.0;

/// A part of a shortcut in the hints: a key, or a word between two keys,
/// such as "to" in "1 to 9".
enum Piece {
    Cap(Arc<egui::Galley>),
    Word(Arc<egui::Galley>),
}

/// Where a galley's first line stands, from its top.
pub(super) fn baseline(galley: &egui::Galley) -> f32 {
    galley
        .rows
        .first()
        .and_then(|row| row.row.glyphs.first().map(|glyph| row.pos.y + glyph.pos.y))
        .unwrap_or(galley.size().y)
}

/// Pairs of keys and what they do, laid out to be painted.
pub(super) struct KeyHints {
    /// Each pair's keys, in their order from the left, and its words.
    pairs: Vec<(Vec<Piece>, Arc<egui::Galley>)>,
    /// The caps' names and what the keys do, in the text color: the hints
    /// are the way to the keys, and gray read as switched off.
    strong: Color32,
    /// A word between two keys.
    weak: Color32,
    dark: bool,
}

impl KeyHints {
    /// `hints`, each its keys and what they do, laid out in `ui`'s
    /// colors; words longer than `room` are cut.
    pub(super) fn new(ui: &Ui, hints: &[(String, String)], room: f32) -> Self {
        let (strong, weak) = (ui.visuals().text_color(), ui.visuals().weak_text_color());
        let font = FontId::proportional(size::SMALL);
        let pairs = hints
            .iter()
            .map(|(keys, what)| {
                let cap = |key: &str| Piece::Cap(shortcut::keycap_label(ui, key, strong));
                // The keys in reading order, in parts that each read left to
                // right: a key combination, a word between keys, a key.
                let (mut units, mut unit) = (Vec::new(), Vec::new());
                for part in keys.split('+').map(str::trim).filter(|k| !k.is_empty()) {
                    // A range such as "1 to 9" is two keys with the word
                    // between. The first key ends the combination before it,
                    // as in Ctrl+1; the word and the last key follow it.
                    let words: Vec<&str> = part.split_whitespace().collect();
                    if words.len() >= 3 {
                        let between = words[1..words.len() - 1].join(" ");
                        let word = one_line(ui, &between, &[], font.clone(), weak, room);
                        unit.push(cap(words[0]));
                        units.push(std::mem::take(&mut unit));
                        units.push(vec![Piece::Word(word)]);
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
                let caps: Vec<Piece> = units.into_iter().flatten().collect();
                (caps, one_line(ui, what, &[], font.clone(), strong, room))
            })
            .collect();
        Self {
            pairs,
            strong,
            weak,
            dark: ui.visuals().dark_mode,
        }
    }

    fn piece_width(piece: &Piece) -> f32 {
        match piece {
            Piece::Cap(g) => shortcut::keycap_size(g, HINT_CAP).x,
            Piece::Word(g) => g.size().x,
        }
    }

    fn caps_width(caps: &[Piece]) -> f32 {
        caps.iter().map(Self::piece_width).sum::<f32>()
            + CAP_GAP * caps.len().saturating_sub(1) as f32
    }

    /// How wide each pair is: its keys, the gap and its words.
    pub(super) fn widths(&self) -> Vec<f32> {
        self.pairs
            .iter()
            .map(|(caps, what)| Self::caps_width(caps) + WORDS_GAP + what.size().x)
            .collect()
    }

    /// Paints pair `n` from `x` points after `within`'s left, mirrored in
    /// Arabic, on the line through `y`.
    pub(super) fn paint(&self, p: &egui::Painter, within: Rect, n: usize, x: f32, y: f32) {
        let Some((caps, what)) = self.pairs.get(n) else {
            return;
        };
        let x = within.left() + x;
        // The keys as one group, placed for Arabic as a whole; its pieces
        // are already in their order from the left.
        let group = flip(
            within,
            Rect::from_min_size(
                Pos2::new(x, y - HINT_CAP / 2.0),
                Vec2::new(Self::caps_width(caps), HINT_CAP),
            ),
        );
        // The line the caps' names stand on, in the middle of the caps;
        // the words stand on it too.
        let base = caps
            .iter()
            .find_map(|piece| match piece {
                Piece::Cap(g) => Some(y - g.size().y / 2.0 + baseline(g)),
                Piece::Word(_) => None,
            })
            .unwrap_or(y);
        let mut cap_x = group.left();
        for piece in caps {
            match piece {
                Piece::Cap(g) => {
                    let size = shortcut::keycap_size(g, HINT_CAP);
                    let cap = Rect::from_min_size(Pos2::new(cap_x, group.top()), size);
                    shortcut::paint_keycap(p, cap, g.clone(), self.strong, self.dark);
                }
                Piece::Word(g) => {
                    p.galley(Pos2::new(cap_x, base - baseline(g)), g.clone(), self.weak);
                }
            }
            cap_x += Self::piece_width(piece) + CAP_GAP;
        }
        let words_x = x + group.width() + WORDS_GAP;
        let at = flip(
            within,
            Rect::from_min_size(Pos2::new(words_x, base - baseline(what)), what.size()),
        );
        p.galley(at.min, what.clone(), self.strong);
    }
}
