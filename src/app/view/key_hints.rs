//! Quiet lines of keys and what they do: the popup's foot and the full
//! window's status bar. The keys show as key caps, as Settings shows them,
//! and the words stand on the line the caps' names stand on.

use super::*;

/// How tall the hints' key caps are: a little smaller than Settings', so
/// two lines of them leave room for the popup's list.
pub(super) const HINT_CAP: f32 = 22.0;

/// The hints' caps: `HINT_CAP` tall, a little closer together.
const HINT_CAPS: shortcut::CapSize = shortcut::CapSize {
    height: HINT_CAP,
    gap: 3.0,
};

/// Between the keys and their words.
const WORDS_GAP: f32 = 6.0;

/// Pairs of keys and what they do, laid out to be painted.
pub(super) struct KeyHints {
    /// Each pair's keys and its words.
    pairs: Vec<(shortcut::Caps, Arc<egui::Galley>)>,
    /// What the keys do, in the text color: the hints are the way to the
    /// keys, and gray read as switched off.
    strong: Color32,
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
                let caps = shortcut::Caps::new(ui, keys, HINT_CAPS, (strong, weak));
                (caps, one_line(ui, what, &[], font.clone(), strong, room))
            })
            .collect();
        Self {
            pairs,
            strong,
            dark: ui.visuals().dark_mode,
        }
    }

    /// How wide each pair is: its keys, the gap and its words.
    pub(super) fn widths(&self) -> Vec<f32> {
        self.pairs
            .iter()
            .map(|(caps, what)| caps.width() + WORDS_GAP + what.size().x)
            .collect()
    }

    /// Paints pair `n` from `x` points after `within`'s left, mirrored in
    /// Arabic, on the line through `y`.
    pub(super) fn paint(&self, p: &egui::Painter, within: Rect, n: usize, x: f32, y: f32) {
        let Some((caps, what)) = self.pairs.get(n) else {
            return;
        };
        let x = within.left() + x;
        // The keys as one group, placed for Arabic as a whole; its parts
        // are already in their order from the left.
        let group = flip(
            within,
            Rect::from_min_size(
                Pos2::new(x, y - HINT_CAP / 2.0),
                Vec2::new(caps.width(), HINT_CAP),
            ),
        );
        caps.paint(p, group.min, self.dark);
        // The words stand on the line the caps' names stand on.
        let base = group.top() + caps.baseline();
        let words_x = x + group.width() + WORDS_GAP;
        let at = flip(
            within,
            Rect::from_min_size(
                Pos2::new(words_x, base - shortcut::baseline(what)),
                what.size(),
            ),
        );
        p.galley(at.min, what.clone(), self.strong);
    }
}
