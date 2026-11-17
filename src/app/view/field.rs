//! Text fields that show right-to-left text correctly (D106).
//!
//! egui 0.37 lays out an Arabic word with more glyphs than letters (each
//! dot is a glyph of its own, and the letters come in screen order), and
//! its text field takes the glyphs for letters: the cursor showed in the
//! middle of a word, End went past the end of the text and Backspace then
//! did nothing. So a field whose text has right-to-left letters hands egui
//! a stand-in instead: one invisible, zero-width glyph per letter. egui's
//! typing, deleting, copying, undo and screen reader support then work on
//! the letters, and this module draws the text in reading order with `bidi`,
//! with the cursor and the selection where they belong. Clicks, drags and
//! the arrow keys go through the same layout, so the arrows move the cursor
//! the way the eye reads it. Text without such letters takes egui's own path.

use std::ops::Range;
use std::sync::Arc;

use eframe::egui::text::{CCursor, CCursorRange};
use eframe::egui::text_edit::TextEditState;
use eframe::egui::text_selection::text_cursor_state::{
    ccursor_next_word, ccursor_previous_word, is_word_char,
};
use eframe::egui::{Align, Align2, Event, Frame, Galley, Id, Modifiers, Pos2, Rect, Ui};
use unicode_bidi::{BidiClass, Level, ParagraphBidiInfo};

use super::*;

/// A text field: `egui::TextEdit` with the options the app uses, which
/// draws right-to-left text itself. Build it like a `TextEdit` and add it
/// with `ui.add`.
pub(crate) struct Field<'t> {
    text: &'t mut String,
    options: Options,
}

/// Everything about a field but its text.
#[derive(Clone)]
struct Options {
    multiline: bool,
    rows: Option<usize>,
    id: Option<Id>,
    hint: Option<egui::Atoms<'static>>,
    width: Option<f32>,
    font: Option<egui::FontSelection>,
    frame: Option<Frame>,
    align: Option<Align2>,
    password: bool,
    text_color: Option<Color32>,
}

impl<'t> Field<'t> {
    pub(crate) fn singleline(text: &'t mut String) -> Self {
        Self::new(text, false)
    }

    pub(crate) fn multiline(text: &'t mut String) -> Self {
        Self::new(text, true)
    }

    fn new(text: &'t mut String, multiline: bool) -> Self {
        Self {
            text,
            options: Options {
                multiline,
                rows: None,
                id: None,
                hint: None,
                width: None,
                font: None,
                frame: None,
                align: None,
                password: false,
                text_color: None,
            },
        }
    }

    pub(crate) fn id(mut self, id: Id) -> Self {
        self.options.id = Some(id);
        self
    }

    pub(crate) fn hint_text(mut self, hint: impl egui::IntoAtoms<'static>) -> Self {
        self.options.hint = Some(hint.into_atoms());
        self
    }

    pub(crate) fn desired_width(mut self, width: f32) -> Self {
        self.options.width = Some(width);
        self
    }

    pub(crate) fn desired_rows(mut self, rows: usize) -> Self {
        self.options.rows = Some(rows);
        self
    }

    pub(crate) fn font(mut self, font: impl Into<egui::FontSelection>) -> Self {
        self.options.font = Some(font.into());
        self
    }

    pub(crate) fn frame(mut self, frame: Frame) -> Self {
        self.options.frame = Some(frame);
        self
    }

    pub(crate) fn align(mut self, align: Align2) -> Self {
        self.options.align = Some(align);
        self
    }

    pub(crate) fn password(mut self, password: bool) -> Self {
        self.options.password = password;
        self
    }

    pub(crate) fn text_color(mut self, color: Color32) -> Self {
        self.options.text_color = Some(color);
        self
    }
}

impl Options {
    /// egui's field over `text`, with these options and the id `id`.
    fn edit<'a>(&self, text: &'a mut String, id: Id) -> egui::TextEdit<'a> {
        let mut edit = if self.multiline {
            egui::TextEdit::multiline(text)
        } else {
            egui::TextEdit::singleline(text)
        }
        .id(id)
        .password(self.password);
        if let Some(rows) = self.rows {
            edit = edit.desired_rows(rows);
        }
        if let Some(hint) = &self.hint {
            edit = edit.hint_text(hint.clone());
        }
        if let Some(width) = self.width {
            edit = edit.desired_width(width);
        }
        if let Some(font) = &self.font {
            edit = edit.font(font.clone());
        }
        if let Some(frame) = self.frame {
            edit = edit.frame(frame);
        }
        if let Some(align) = self.align {
            edit = edit.align(align);
        }
        if let Some(color) = self.text_color {
            edit = edit.text_color(color);
        }
        edit
    }
}

impl egui::Widget for Field<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let Field { text, options } = self;
        // egui's own id for the field, on both paths, so a field keeps its
        // cursor when its text gains or loses its right-to-left letters.
        let id = options.id.unwrap_or_else(|| ui.next_auto_id());
        // Also from the frame that types the first such letter.
        let focused = ui.memory(|m| m.has_focus(id));
        let typed_rtl = focused && ui.input(|i| i.events.iter().any(adds_rtl));
        let drawn = !options.password && (bidi::has_rtl(text) || typed_rtl);
        if drawn {
            drawn_field(ui, text, &options, id)
        } else {
            ui.add(options.edit(text, id))
        }
    }
}

/// `event` puts right-to-left letters in the field.
fn adds_rtl(event: &Event) -> bool {
    match event {
        Event::Text(text) | Event::Paste(text) => bidi::has_rtl(text),
        Event::Ime(egui::ImeEvent::Commit(text)) => bidi::has_rtl(text),
        _ => false,
    }
}

/// A field whose text has right-to-left letters: egui edits a stand-in and
/// this draws the text, the selection and the cursor.
fn drawn_field(ui: &mut Ui, text: &mut String, options: &Options, id: Id) -> egui::Response {
    let font = options.font.clone().unwrap_or_default().resolve(ui.style());
    let color = options
        .text_color
        .or(ui.visuals().override_text_color)
        .unwrap_or_else(|| ui.visuals().widgets.inactive.text_color());
    let width_id = id.with("text-width");
    let wrap = if options.multiline {
        ui.data(|d| d.get_temp::<f32>(width_id))
            .unwrap_or_else(|| options.width.unwrap_or(ui.spacing().text_edit_width) - 8.0)
    } else {
        f32::INFINITY
    };
    let focused = ui.memory(|m| m.has_focus(id));
    // The keys that move the cursor go by what is seen.
    let mut before = Drawn::new(ui, text, &font, color, wrap);
    if options.multiline {
        // Each line against its own edge, as drawn, for Up and Down.
        let room = Rect::from_min_size(Pos2::ZERO, Vec2::new(wrap, 0.0));
        before.place(room, 0.0, Align2::LEFT_TOP, true, |_, _| None, None);
    }
    if focused {
        take_keys(ui, id, text, &before, options.multiline);
    }
    let edit = options.edit(text, id);
    let multiline = options.multiline;
    let mut stand_in_layouter = |ui: &Ui, buffer: &dyn egui::TextBuffer, wrap: f32| {
        stand_in(ui, buffer.as_str(), &font, color, multiline.then_some(wrap))
    };
    let output = {
        let visuals = ui.visuals_mut();
        let (cursor, selection) = (visuals.text_cursor.stroke, visuals.selection.bg_fill);
        // egui's own cursor and selection would sit on the stand-in.
        visuals.text_cursor.stroke.color = Color32::TRANSPARENT;
        visuals.selection.bg_fill = Color32::TRANSPARENT;
        let output = edit.layouter(&mut stand_in_layouter).show(ui);
        let visuals = ui.visuals_mut();
        visuals.text_cursor.stroke = cursor;
        visuals.selection.bg_fill = selection;
        output
    };
    let response = output.response.response;
    let clip = output.text_clip_rect;
    if options.multiline {
        ui.data_mut(|d| d.insert_temp(width_id, clip.width()));
    }
    let wrap = if options.multiline {
        clip.width()
    } else {
        f32::INFINITY
    };
    let mut drawn = Drawn::new(ui, text, &font, color, wrap);
    let focused = ui.memory(|m| m.has_focus(id));
    let mut state = output.state;
    let mut range = state.cursor.char_range();

    // Where the lines go, the one line scrolled to keep the cursor in view.
    let align = options.align.unwrap_or(Align2::LEFT_TOP);
    let scroll_id = id.with("scroll");
    let mut scroll = ui.data(|d| d.get_temp::<f32>(scroll_id)).unwrap_or(0.0);
    drawn.place(
        clip,
        output.galley_pos.y,
        align,
        options.multiline,
        |line, caret| {
            let room = clip.width();
            if line.width <= room {
                scroll = 0.0;
                return None;
            }
            let most = line.width - room;
            match caret {
                Some(x) => {
                    if x < scroll {
                        scroll = x;
                    } else if x > scroll + room {
                        scroll = x - room;
                    }
                }
                // Without the cursor, the start of the text shows.
                None => scroll = if line.rtl { most } else { 0.0 },
            }
            scroll = scroll.clamp(0.0, most);
            Some(clip.left() - scroll)
        },
        focused.then_some(range).flatten(),
    );
    ui.data_mut(|d| d.insert_temp(scroll_id, scroll));

    // Clicks and drags put the cursor under the pointer.
    if let Some(pointer) = ui.input(|i| i.pointer.interact_pos()) {
        let mut picked = None;
        if response.double_clicked() {
            let at = drawn.boundary_at(pointer);
            picked = Some(word_around(text, at));
        } else if response.triple_clicked() {
            picked = Some(CCursorRange::select_all_str(text));
        } else if response.is_pointer_button_down_on() || response.clicked() {
            // A click can start and end in one frame (a touchpad tap):
            // egui has then forgotten where it started, which is here.
            let origin = ui.input(|i| i.pointer.press_origin()).unwrap_or(pointer);
            let focus = CCursor::new(drawn.boundary_at(pointer));
            let shift = ui.input(|i| i.modifiers.shift);
            let anchor = match range {
                Some(range) if shift => range.secondary,
                _ => CCursor::new(drawn.boundary_at(origin)),
            };
            picked = Some(CCursorRange::two(anchor, focus));
        }
        if let Some(picked) = picked {
            range = Some(picked);
            state.cursor.set_char_range(range);
            state.store(ui.ctx(), id);
        }
    }

    // The cursor blinks from its last move, as egui's does.
    let moved_id = id.with("moved");
    let now = ui.input(|i| i.time);
    let (mut moved_at, last_range, last_len) = ui
        .data(|d| d.get_temp::<(f64, Option<CCursorRange>, usize)>(moved_id))
        .unwrap_or((now, range, text.len()));
    if range != last_range || text.len() != last_len || response.changed() {
        moved_at = now;
    }
    ui.data_mut(|d| d.insert_temp(moved_id, (moved_at, range, text.len())));

    if ui.is_rect_visible(clip) {
        let painter = ui.painter_at(clip.expand(1.0));
        let selected = range.filter(|range| focused && !range.is_empty());
        if let Some(range) = selected {
            let [start, end] = range.sorted_cursors();
            for rect in drawn.selection(start.index.0..end.index.0) {
                painter.rect_filled(rect, CornerRadius::ZERO, ui.visuals().selection.bg_fill);
            }
        }
        for line in &drawn.lines {
            for (x, galley) in &line.pieces {
                let at = Pos2::new(line.left + x, line.top);
                painter.galley(at, Arc::clone(galley), color);
            }
        }
        // Only while the window has the keyboard, as egui's.
        let shown = range.filter(|_| focused && ui.input(|i| i.focused));
        if let Some(range) = shown {
            let caret = drawn.caret_rect(range.primary.index.0);
            egui::text_selection::visuals::paint_text_cursor(ui, &painter, caret, now - moved_at);
            // An input method puts its window by the cursor.
            let to_global = ui
                .ctx()
                .layer_transform_to_global(ui.layer_id())
                .unwrap_or_default();
            ui.ctx().output_mut(|o| {
                if let Some(ime) = &mut o.ime {
                    ime.cursor_rect = to_global * caret;
                }
            });
        }
    }
    response
}

/// egui's stand-in for `text`: one invisible, zero-width glyph per letter,
/// and the line breaks. Screen readers read the letters from the glyphs.
/// It is as big as the text drawn, in several lines across `wrap`: egui
/// gives the field room for it and shows only what is inside its place.
fn stand_in(ui: &Ui, text: &str, font: &FontId, color: Color32, wrap: Option<f32>) -> Arc<Galley> {
    let shown: String = text
        .chars()
        .map(|c| if c == '\n' { '\n' } else { '\u{200B}' })
        .collect();
    let job =
        egui::text::LayoutJob::simple(shown, font.clone(), Color32::TRANSPARENT, f32::INFINITY);
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let mut letters = text.chars().filter(|&c| c != '\n');
    let mut galley = (*galley).clone();
    for placed in &mut galley.rows {
        let row = Arc::make_mut(&mut placed.row);
        for glyph in &mut row.glyphs {
            if let Some(letter) = letters.next() {
                glyph.chr = letter;
            }
        }
    }
    let drawn = Drawn::new(ui, text, font, color, wrap.unwrap_or(f32::INFINITY));
    let widest = drawn
        .lines
        .iter()
        .map(|line| line.width)
        .fold(0.0, f32::max);
    // Several lines take the whole width: each sits against its own edge.
    let width = wrap.filter(|wrap| wrap.is_finite()).unwrap_or(widest);
    let height = drawn.lines.len() as f32 * drawn.row_height;
    galley.rect = Rect::from_min_size(galley.rect.min, Vec2::new(width, height));
    Arc::new(galley)
}

/// Takes the keys that move the cursor from egui and moves it by what is
/// seen: left and right on the screen, and in several lines up, down and to
/// the line's ends.
fn take_keys(ui: &mut Ui, id: Id, text: &str, drawn: &Drawn, multiline: bool) {
    let keys: Vec<(Key, Modifiers)> = ui.input_mut(|i| {
        let mut taken = Vec::new();
        i.events.retain(|event| match event {
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } if moves_cursor(*key, *modifiers, multiline) => {
                taken.push((*key, *modifiers));
                false
            }
            _ => true,
        });
        taken
    });
    if keys.is_empty() {
        return;
    }
    let mut state = TextEditState::load(ui.ctx(), id).unwrap_or_default();
    let end = CCursor::new(text.chars().count());
    let mut range = state.cursor.char_range().unwrap_or(CCursorRange::one(end));
    for (key, modifiers) in keys {
        range = moved(drawn, text, range, key, modifiers);
    }
    state.cursor.set_char_range(Some(range));
    state.store(ui.ctx(), id);
}

fn moves_cursor(key: Key, modifiers: Modifiers, multiline: bool) -> bool {
    if modifiers.alt || modifiers.mac_cmd {
        return false;
    }
    match key {
        Key::ArrowLeft | Key::ArrowRight => true,
        // Ctrl+Home and Ctrl+End stay egui's: the start and the end.
        Key::ArrowUp | Key::ArrowDown | Key::Home | Key::End => multiline && !modifiers.command,
        _ => false,
    }
}

/// `range` after `key`: Shift moves only the cursor's end, Ctrl moves by
/// words.
fn moved(
    drawn: &Drawn,
    text: &str,
    range: CCursorRange,
    key: Key,
    modifiers: Modifiers,
) -> CCursorRange {
    let at = range.primary.index.0;
    let to = match key {
        Key::ArrowLeft | Key::ArrowRight => {
            let left = key == Key::ArrowLeft;
            if !modifiers.shift && !range.is_empty() {
                // A selection first closes to its end on that side.
                let [a, b] = range.sorted_cursors().map(|c| c.index.0);
                let (xa, xb) = (drawn.caret_x(a), drawn.caret_x(b));
                if (xa < xb) == left {
                    a
                } else {
                    b
                }
            } else if modifiers.command {
                drawn.word_step(text, at, left)
            } else {
                drawn.step(at, left)
            }
        }
        Key::ArrowUp => drawn.vertical(at, true),
        Key::ArrowDown => drawn.vertical(at, false),
        Key::Home => drawn.line_of(at).chars.start,
        Key::End => drawn.line_of(at).chars.end,
        _ => at,
    };
    if modifiers.shift {
        CCursorRange::two(range.secondary, CCursor::new(to))
    } else {
        CCursorRange::one(CCursor::new(to))
    }
}

/// The word around the letter boundary `at`, as double-clicking selects it.
fn word_around(text: &str, at: usize) -> CCursorRange {
    let chars: Vec<char> = text.chars().collect();
    let in_word = |c: char| is_word_char(c) || is_mark(c);
    let mut start = at.min(chars.len());
    let mut end = start;
    while start > 0 && in_word(chars[start - 1]) {
        start -= 1;
    }
    while end < chars.len() && in_word(chars[end]) {
        end += 1;
    }
    CCursorRange::two(CCursor::new(start), CCursor::new(end))
}

/// A combining mark, such as an Arabic diacritic: it sits on its letter.
fn is_mark(c: char) -> bool {
    unicode_bidi::bidi_class(c) == BidiClass::NSM
}

/// The text as it is drawn: its lines in reading order, and where each
/// letter is.
struct Drawn {
    lines: Vec<Line>,
    row_height: f32,
}

/// One drawn line.
struct Line {
    /// Its letters, by their place in the text (counted in chars).
    chars: Range<usize>,
    /// It reads right to left.
    rtl: bool,
    width: f32,
    /// Where it is drawn: its left edge and its top.
    left: f32,
    top: f32,
    /// Its pieces in reading order, each at its place from the line's left.
    pieces: Vec<(f32, Arc<Galley>)>,
    /// Each letter's place from the line's left.
    boxes: Vec<LetterBox>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct LetterBox {
    left: f32,
    right: f32,
    /// It reads right to left.
    rtl: bool,
    /// A combining mark on the letter before it.
    mark: bool,
}

impl Drawn {
    /// `text` in `font` and `color`, its paragraphs wrapped at `wrap`.
    fn new(ui: &Ui, text: &str, font: &FontId, color: Color32, wrap: f32) -> Self {
        let ui_rtl = crate::i18n::rtl();
        let mut lines = Vec::new();
        let mut paragraph_start = 0;
        for paragraph in text.split('\n') {
            let rtl = bidi::starts_rtl(paragraph, ui_rtl);
            for bytes in wrap_ranges(ui, paragraph, font, wrap) {
                let start = paragraph_start + paragraph[..bytes.start].chars().count();
                lines.push(Line::new(ui, &paragraph[bytes], start, rtl, font, color));
            }
            paragraph_start += paragraph.chars().count() + 1;
        }
        let row_height = ui.fonts_mut(|f| f.row_height(font));
        Self { lines, row_height }
    }

    /// Places the lines in `clip` from `top`: one line by `align`, or at the
    /// left that `overflow` gives when it is wider than `clip` (it gets the
    /// line and the cursor's place on it); several lines each against its
    /// own edge.
    fn place(
        &mut self,
        clip: Rect,
        top: f32,
        align: Align2,
        multiline: bool,
        mut overflow: impl FnMut(&Line, Option<f32>) -> Option<f32>,
        range: Option<CCursorRange>,
    ) {
        let row_height = self.row_height;
        let caret = range.map(|r| r.primary.index.0);
        let caret_x = caret.map(|at| self.caret_x_in_line(at));
        for (n, line) in self.lines.iter_mut().enumerate() {
            line.top = top + n as f32 * row_height;
            line.left = if multiline {
                if line.rtl {
                    clip.right() - line.width
                } else {
                    clip.left()
                }
            } else if let Some(left) = overflow(line, caret_x) {
                left
            } else {
                match align.x() {
                    Align::Min => clip.left(),
                    Align::Center => clip.center().x - line.width / 2.0,
                    Align::Max => clip.right() - line.width,
                }
            };
        }
    }

    /// The line that shows the boundary `at`: at a line break, the next line.
    fn line_index(&self, at: usize) -> usize {
        self.lines
            .iter()
            .rposition(|line| line.chars.start <= at)
            .unwrap_or(0)
    }

    fn line_of(&self, at: usize) -> &Line {
        &self.lines[self.line_index(at)]
    }

    /// Where the cursor at the boundary `at` is, from its line's left.
    fn caret_x_in_line(&self, at: usize) -> f32 {
        let line = self.line_of(at);
        line.caret_x(at.saturating_sub(line.chars.start).min(line.boxes.len()))
    }

    /// Where the cursor at the boundary `at` is on the screen.
    fn caret_x(&self, at: usize) -> f32 {
        self.line_of(at).left + self.caret_x_in_line(at)
    }

    fn caret_rect(&self, at: usize) -> Rect {
        let line = self.line_of(at);
        let x = self.caret_x(at);
        Rect::from_min_max(
            Pos2::new(x, line.top),
            Pos2::new(x, line.top + self.row_height),
        )
    }

    /// The boundary under `pointer`.
    fn boundary_at(&self, pointer: Pos2) -> usize {
        let n = (((pointer.y - self.lines[0].top) / self.row_height)
            .floor()
            .max(0.0) as usize)
            .min(self.lines.len() - 1);
        let line = &self.lines[n];
        line.chars.start + line.boundary_at(pointer.x - line.left)
    }

    /// The boundary next to `at` on the screen, to the left or the right;
    /// past a line's edge, the next line in reading order.
    fn step(&self, at: usize, left: bool) -> usize {
        let n = self.line_index(at);
        let line = &self.lines[n];
        let local = at.saturating_sub(line.chars.start).min(line.boxes.len());
        if let Some(next) = line.step(local, left) {
            return line.chars.start + next;
        }
        // Left goes on reading in a right-to-left line, back in another.
        let forward = left == line.rtl;
        match (forward, self.lines.get(n + 1), n.checked_sub(1)) {
            (true, Some(next), _) => next.chars.start,
            (false, _, Some(previous)) => self.lines[previous].chars.end,
            _ => at,
        }
    }

    /// The boundary a word away from `at`, to the left or the right.
    fn word_step(&self, text: &str, at: usize, left: bool) -> usize {
        let forward = left == self.line_of(at).rtl;
        let cursor = CCursor::new(at);
        if forward {
            ccursor_next_word(text, cursor).index.0
        } else {
            ccursor_previous_word(text, cursor).index.0
        }
    }

    /// The boundary on the line above or below `at` nearest to it on the
    /// screen; past the first line its start, past the last its end.
    fn vertical(&self, at: usize, up: bool) -> usize {
        let n = self.line_index(at);
        let target = if up { n.checked_sub(1) } else { Some(n + 1) };
        let Some(line) = target.and_then(|t| self.lines.get(t)) else {
            let line = &self.lines[n];
            return if up { line.chars.start } else { line.chars.end };
        };
        line.chars.start + line.boundary_at(self.caret_x(at) - line.left)
    }

    /// The rectangles that show the letters of `range` (in chars) selected.
    fn selection(&self, range: Range<usize>) -> Vec<Rect> {
        let mut rects: Vec<Rect> = Vec::new();
        for line in &self.lines {
            let mut spans: Vec<(f32, f32)> = line
                .boxes
                .iter()
                .enumerate()
                .filter(|(i, b)| range.contains(&(line.chars.start + i)) && b.right > b.left)
                .map(|(_, b)| (b.left, b.right))
                .collect();
            spans.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut merged: Vec<(f32, f32)> = Vec::new();
            for (left, right) in spans {
                match merged.last_mut() {
                    Some(last) if left <= last.1 + 0.5 => last.1 = last.1.max(right),
                    _ => merged.push((left, right)),
                }
            }
            for (left, right) in merged {
                rects.push(Rect::from_min_max(
                    Pos2::new(line.left + left, line.top),
                    Pos2::new(line.left + right, line.top + self.row_height),
                ));
            }
        }
        rects
    }
}

impl Line {
    /// The line `text`, whose first letter is letter `start` of the field.
    fn new(ui: &Ui, text: &str, start: usize, rtl: bool, font: &FontId, color: Color32) -> Self {
        let starts: Vec<usize> = text.char_indices().map(|(at, _)| at).collect();
        let chars: Vec<char> = text.chars().collect();
        let mut boxes = vec![LetterBox::default(); chars.len()];
        let mut pieces = Vec::new();
        let mut x = 0.0;
        if !text.is_empty() {
            let level = if rtl { Level::rtl() } else { Level::ltr() };
            let info = ParagraphBidiInfo::new(text, Some(level));
            let (levels, runs) = info.visual_runs(0..text.len());
            for run in runs {
                let run_rtl = levels[run.start].is_rtl();
                let mut run_pieces = if run_rtl {
                    bidi::rtl_pieces(text, run.clone())
                } else {
                    vec![(run.clone(), text[run.clone()].to_string())]
                };
                if run_rtl {
                    run_pieces.reverse();
                }
                for (bytes, shown) in run_pieces {
                    let galley = ui.fonts_mut(|f| f.layout_no_wrap(shown, font.clone(), color));
                    let first = starts.partition_point(|&at| at < bytes.start);
                    let end = starts.partition_point(|&at| at < bytes.end);
                    // A right-to-left word's glyphs come in screen order.
                    let reversed = run_rtl && end - first > 1;
                    place_letters(
                        &galley,
                        &chars[first..end],
                        reversed,
                        x,
                        &mut boxes[first..end],
                    );
                    x += galley.size().x;
                    pieces.push((x - galley.size().x, galley));
                }
            }
            for (i, letter) in boxes.iter_mut().enumerate() {
                letter.rtl = levels[starts[i]].is_rtl();
                letter.mark = is_mark(chars[i]);
            }
        }
        Self {
            chars: start..start + chars.len(),
            rtl,
            width: x,
            left: 0.0,
            top: 0.0,
            pieces,
            boxes,
        }
    }

    /// Where the cursor at the boundary `at` (counted from the line's start)
    /// is from the line's left: after a right-to-left letter on its left, after
    /// another on its right.
    fn caret_x(&self, at: usize) -> f32 {
        if self.boxes.is_empty() {
            return 0.0;
        }
        if at == 0 {
            let first = self.boxes[0];
            return if first.rtl { first.right } else { first.left };
        }
        let before = self.boxes[at - 1];
        if before.rtl {
            before.left
        } else {
            before.right
        }
    }

    /// The boundary under `x` (from the line's left): before or after the
    /// letter there, by the half of it that `x` is in.
    fn boundary_at(&self, x: f32) -> usize {
        let wide = self
            .boxes
            .iter()
            .enumerate()
            .filter(|(_, b)| b.right > b.left);
        let nearest = wide.min_by(|(_, a), (_, b)| distance(a, x).total_cmp(&distance(b, x)));
        let Some((i, letter)) = nearest else {
            return 0;
        };
        let middle = (letter.left + letter.right) / 2.0;
        let before = if letter.rtl { x >= middle } else { x < middle };
        let mut at = if before { i } else { i + 1 };
        // A mark stays with its letter.
        while at < self.boxes.len() && self.boxes[at].mark {
            at += 1;
        }
        at
    }

    /// The boundary next to `at` on the screen, to the left or the right, on
    /// this line: the nearest farther one, and of several in one place the
    /// nearest in the text. None at the line's edge.
    fn step(&self, at: usize, left: bool) -> Option<usize> {
        let x = self.caret_x(at);
        let mut best: Option<(f32, usize)> = None;
        for other in 0..=self.boxes.len() {
            // Not between a letter and its mark.
            if other == at || self.boxes.get(other).is_some_and(|b| b.mark) {
                continue;
            }
            let other_x = self.caret_x(other);
            let farther = if left {
                other_x < x - 0.5
            } else {
                other_x > x + 0.5
            };
            if !farther {
                continue;
            }
            let better = match best {
                None => true,
                Some((best_x, best_at)) => {
                    let nearer = if left {
                        other_x > best_x + 0.5
                    } else {
                        other_x < best_x - 0.5
                    };
                    let same_place = (other_x - best_x).abs() <= 0.5;
                    nearer || (same_place && other.abs_diff(at) < best_at.abs_diff(at))
                }
            };
            if better {
                best = Some((other_x, other));
            }
        }
        best.map(|(_, other)| other)
    }
}

fn distance(letter: &LetterBox, x: f32) -> f32 {
    if x < letter.left {
        letter.left - x
    } else if x > letter.right {
        x - letter.right
    } else {
        0.0
    }
}

/// Puts `letters`, drawn as `galley` at `x`, in `boxes`. Each letter takes a
/// glyph that has width, in order (`reversed` for a right-to-left word, whose
/// glyphs come in screen order); marks and the dots egui gives glyphs of their
/// own have none. When the glyphs don't match the letters, each letter gets
/// an even share of the piece.
fn place_letters(
    galley: &Galley,
    letters: &[char],
    reversed: bool,
    x: f32,
    boxes: &mut [LetterBox],
) {
    let width = galley.size().x;
    let mut wide: Vec<(f32, f32)> = galley
        .rows
        .first()
        .map(|placed| {
            placed
                .row
                .glyphs
                .iter()
                .filter(|g| g.advance_width > 0.0)
                .map(|g| {
                    (
                        placed.pos.x + g.pos.x,
                        placed.pos.x + g.pos.x + g.advance_width,
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let count = letters.iter().filter(|&&c| !is_mark(c)).count();
    if wide.len() != count {
        let share = width / count.max(1) as f32;
        wide = (0..count)
            .map(|k| (k as f32 * share, (k + 1) as f32 * share))
            .collect();
    }
    if reversed {
        wide.reverse();
    }
    let mut spans = wide.into_iter();
    let mut last: Option<(f32, f32)> = None;
    for (letter, place) in letters.iter().zip(boxes.iter_mut()) {
        let (left, right) = if is_mark(*letter) {
            // On its letter: where the cursor goes after the letter.
            let at = match last {
                Some((left, right)) => {
                    if reversed {
                        left
                    } else {
                        right
                    }
                }
                None => {
                    if reversed {
                        width
                    } else {
                        0.0
                    }
                }
            };
            (at, at)
        } else {
            let span = spans.next().unwrap_or((width, width));
            last = Some(span);
            span
        };
        place.left = x + left;
        place.right = x + right;
    }
}

/// The byte ranges of `paragraph`'s lines when wrapped at `width`: at the
/// spaces, which stay at the end of their line, or inside a word too long
/// for a line. Every letter is on a line; an empty paragraph is one empty
/// line.
fn wrap_ranges(ui: &Ui, paragraph: &str, font: &FontId, width: f32) -> Vec<Range<usize>> {
    if !width.is_finite() || paragraph.is_empty() {
        let whole = 0..paragraph.len();
        return vec![whole];
    }
    let measure = |s: &str| {
        ui.fonts_mut(|f| {
            f.layout_no_wrap(s.to_string(), font.clone(), Color32::WHITE)
                .size()
                .x
        })
    };
    let mut lines = Vec::new();
    let mut start = 0;
    let mut end = 0;
    for word in paragraph.split_inclusive(' ') {
        let word_end = end + word.len();
        if end > start && measure(paragraph[start..word_end].trim_end()) > width {
            lines.push(start..end);
            start = end;
        }
        end = word_end;
        // A word wider than a line breaks inside.
        while measure(paragraph[start..end].trim_end()) > width
            && paragraph[start..end].chars().count() > 1
        {
            let mut cut = end;
            while cut > start && measure(&paragraph[start..cut]) > width {
                cut = paragraph[start..cut]
                    .char_indices()
                    .last()
                    .map_or(start, |(at, _)| start + at);
            }
            let first_char = paragraph[start..].chars().next().map_or(1, char::len_utf8);
            let cut = cut.max(start + first_char);
            lines.push(start..cut);
            start = cut;
        }
    }
    lines.push(start..end);
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A window with the app's fonts.
    fn window() -> egui::Context {
        let ctx = egui::Context::default();
        add_fonts(&ctx);
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .textures_delta
            .clear();
        ctx
    }

    fn font() -> FontId {
        FontId::proportional(16.0)
    }

    /// Every text box in the window is a `Field` (D106): egui's own one
    /// shows the cursor in the wrong place in Arabic.
    #[test]
    fn every_text_box_is_a_field() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/app");
        let mut found = Vec::new();
        let mut paths = vec![root];
        while let Some(path) = paths.pop() {
            if path.is_dir() {
                for entry in std::fs::read_dir(&path).unwrap() {
                    paths.push(entry.unwrap().path());
                }
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") || path.ends_with("view/field.rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            // Tests may use egui's: the code ends where its test module starts.
            let code = text
                .match_indices("#[cfg(test)]")
                .find(|(at, m)| text[at + m.len()..].trim_start().starts_with("mod "))
                .map_or(text.as_str(), |(at, _)| &text[..at]);
            for (n, line) in code.lines().enumerate() {
                if line.contains("TextEdit::singleline") || line.contains("TextEdit::multiline") {
                    found.push(format!("{}:{}: {}", path.display(), n + 1, line.trim()));
                }
            }
        }
        assert!(found.is_empty(), "egui's text box:\n{}", found.join("\n"));
    }

    /// `text` drawn as one line in a field.
    fn drawn(ctx: &egui::Context, text: &str) -> Drawn {
        let mut drawn = None;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            drawn = Some(Drawn::new(ui, text, &font(), Color32::WHITE, f32::INFINITY));
        });
        output.textures_delta.clear();
        drawn.expect("drawn")
    }

    #[test]
    fn the_stand_in_has_one_glyph_per_letter() {
        let ctx = window();
        let text = "اجتماع غدا\nmeeting";
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let galley = stand_in(ui, text, &font(), Color32::WHITE, None);
            assert_eq!(galley.end().index.0, text.chars().count());
            // Invisible letters, in the room the text takes as drawn.
            let glyphs = || galley.rows.iter().flat_map(|r| r.glyphs.iter());
            assert!(glyphs().all(|g| g.advance_width == 0.0));
            let drawn = Drawn::new(ui, text, &font(), Color32::WHITE, f32::INFINITY);
            let widest = drawn.lines.iter().map(|l| l.width).fold(0.0, f32::max);
            assert!(widest > 0.0);
            assert_eq!(galley.size(), Vec2::new(widest, 2.0 * drawn.row_height));
            let letters: String = galley
                .rows
                .iter()
                .flat_map(|r| r.glyphs.iter().map(|g| g.chr))
                .collect();
            assert_eq!(letters, text.replace('\n', ""));
        });
        output.textures_delta.clear();
    }

    /// In Arabic the cursor goes from the right to the left as the text
    /// goes on, in English from the left to the right.
    #[test]
    fn the_cursor_follows_the_reading_order() {
        let ctx = window();
        for (text, rtl) in [("اجتماع غدا", true), ("meeting notes", false)] {
            let drawn = drawn(&ctx, text);
            let line = &drawn.lines[0];
            let places: Vec<f32> = (0..=line.boxes.len()).map(|at| line.caret_x(at)).collect();
            for pair in places.windows(2) {
                if rtl {
                    assert!(pair[1] <= pair[0], "{text}: {places:?}");
                } else {
                    assert!(pair[1] >= pair[0], "{text}: {places:?}");
                }
            }
            let (start, end) = (places[0], places[places.len() - 1]);
            if rtl {
                assert!(
                    (start - line.width).abs() < 1.0 && end.abs() < 1.0,
                    "{places:?}"
                );
            } else {
                assert!(
                    start.abs() < 1.0 && (end - line.width).abs() < 1.0,
                    "{places:?}"
                );
            }
        }
    }

    /// English inside an Arabic line reads left to right, and the Arabic
    /// around it right to left.
    #[test]
    fn mixed_text_keeps_each_direction() {
        let ctx = window();
        let text = "فاتورة March 2026";
        let drawn = drawn(&ctx, text);
        let line = &drawn.lines[0];
        let chars: Vec<char> = text.chars().collect();
        let at = |c: char| line.boxes[chars.iter().position(|&x| x == c).unwrap()];
        // "March" reads left to right, and sits left of "فاتورة".
        assert!(at('M').left < at('a').left && at('a').left < at('h').left);
        assert!(at('h').right <= at('ف').left + 0.5);
        assert!(at('ف').left > at('ة').left, "فاتورة reads right to left");
        // No two letters overlap, but for joined Arabic letters touching
        // by a pixel or so.
        let mut spans: Vec<(f32, f32)> = line
            .boxes
            .iter()
            .filter(|b| b.right > b.left)
            .map(|b| (b.left, b.right))
            .collect();
        spans.sort_by(|a, b| a.0.total_cmp(&b.0));
        for pair in spans.windows(2) {
            assert!(pair[1].0 >= pair[0].1 - 1.5, "{spans:?}");
        }
    }

    /// The arrows move the cursor the way the eye reads: Left goes on in
    /// Arabic and back in English.
    #[test]
    fn the_arrows_move_with_the_eye() {
        let ctx = window();
        let arabic = drawn(&ctx, "اجتماع غدا");
        assert_eq!(arabic.step(0, true), 1);
        assert_eq!(arabic.step(4, true), 5);
        assert_eq!(arabic.step(5, false), 4);
        assert_eq!(arabic.step(10, true), 10, "the line's left edge");
        let english = drawn(&ctx, "meeting");
        assert_eq!(english.step(3, true), 2);
        assert_eq!(english.step(3, false), 4);
        assert_eq!(english.step(0, true), 0);
    }

    /// A click puts the cursor where it lands: at the right end of an
    /// Arabic line is its start.
    #[test]
    fn a_click_puts_the_cursor_under_the_pointer() {
        let ctx = window();
        let drawn = drawn(&ctx, "اجتماع غدا");
        let line = &drawn.lines[0];
        let y = line.top + 5.0;
        assert_eq!(drawn.boundary_at(Pos2::new(line.width + 20.0, y)), 0);
        assert_eq!(drawn.boundary_at(Pos2::new(-20.0, y)), 10);
        let middle = drawn.boundary_at(Pos2::new(line.caret_x(3), y));
        assert_eq!(middle, 3);
    }

    #[test]
    fn long_text_wraps_without_losing_letters() {
        let ctx = window();
        let text = "اقرأ كل النص الموجود في الصورة كما هو، سطرًا بسطر، من غير شرح";
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let drawn = Drawn::new(ui, text, &font(), Color32::WHITE, 120.0);
            assert!(drawn.lines.len() > 2, "{}", drawn.lines.len());
            let mut next = 0;
            for line in &drawn.lines {
                assert_eq!(line.chars.start, next);
                assert_eq!(line.boxes.len(), line.chars.len());
                next = line.chars.end;
            }
            assert_eq!(next, text.chars().count());
            // Up and Down go to the line above and below.
            let second = drawn.lines[1].chars.start + 2;
            assert!(drawn.lines[0].chars.contains(&drawn.vertical(second, true)));
            assert!(drawn.lines[2]
                .chars
                .contains(&drawn.vertical(second, false)));
        });
        output.textures_delta.clear();
    }

    /// Typing into a field, with the keys and what the field holds after.
    struct Typing {
        ctx: egui::Context,
        text: String,
        id: Id,
        time: f64,
    }

    impl Typing {
        fn new() -> Self {
            let mut typing = Self {
                ctx: window(),
                text: String::new(),
                id: Id::unique("typing"),
                time: 0.0,
            };
            let id = typing.id;
            typing.frame(Vec::new(), |ui| ui.memory_mut(|m| m.request_focus(id)));
            typing
        }

        /// One frame with `events`; `then` runs after the field.
        fn frame(&mut self, events: Vec<Event>, mut then: impl FnMut(&mut Ui)) -> egui::FullOutput {
            self.time += 1.0;
            let input = egui::RawInput {
                events,
                focused: true,
                time: Some(self.time),
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(600.0, 200.0))),
                ..Default::default()
            };
            let (text, id) = (&mut self.text, self.id);
            let mut output = self.ctx.run_ui(input, |ui| {
                ui.add(Field::singleline(&mut *text).id(id).desired_width(300.0));
                then(ui);
            });
            output.textures_delta.clear();
            output
        }

        fn key(&mut self, key: Key) -> egui::FullOutput {
            self.frame(
                vec![Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }],
                |_| {},
            )
        }

        fn write(&mut self, text: &str) -> egui::FullOutput {
            self.frame(vec![Event::Text(text.into())], |_| {})
        }
    }

    /// The keys from the check in the window: End and Backspace work on
    /// Arabic, Home goes to its start, and the arrows move with the eye.
    #[test]
    fn keys_work_on_arabic() {
        let mut typing = Typing::new();
        typing.write("اجتماع");
        typing.key(Key::End);
        typing.key(Key::Backspace);
        assert_eq!(typing.text, "اجتما");
        typing.key(Key::Home);
        typing.write("ب");
        assert_eq!(typing.text, "باجتما");
        // Left goes on in Arabic: past ا.
        typing.key(Key::ArrowLeft);
        typing.write("و");
        assert_eq!(typing.text, "باوجتما");
        // Right goes back in Arabic: before و, then before ا.
        typing.key(Key::ArrowRight);
        typing.key(Key::ArrowRight);
        typing.key(Key::Delete);
        assert_eq!(typing.text, "بوجتما");
        // English still takes egui's path.
        let mut typing = Typing::new();
        typing.write("meeting");
        // Left goes back in English: before g, so Backspace takes the n.
        typing.key(Key::ArrowLeft);
        typing.key(Key::Backspace);
        assert_eq!(typing.text, "meetig");
    }

    /// The text `output` drew: where each piece is, and what it is clipped
    /// to. The stand-in egui draws is invisible and doesn't count.
    fn text_shown(output: &egui::FullOutput) -> Vec<(Rect, Rect)> {
        let stand_in = |galley: &Galley| {
            let text = &galley.job.text;
            text.chars().all(|c| c == '\u{200B}' || c == '\n')
        };
        output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape)
                    if shape.galley.size().x > 0.0 && !stand_in(&shape.galley) =>
                {
                    let rect = shape.galley.rect.translate(shape.pos.to_vec2());
                    Some((rect, clipped.clip_rect))
                }
                _ => None,
            })
            .collect()
    }

    /// Where `output` drew the text cursor, and the text's left and right.
    /// egui's own cursor, on the stand-in, is invisible and doesn't count.
    fn cursor_and_text(output: &egui::FullOutput) -> (Option<f32>, f32, f32) {
        let mut text_left = f32::INFINITY;
        let mut text_right = f32::NEG_INFINITY;
        for (rect, _) in text_shown(output) {
            text_left = text_left.min(rect.left());
            text_right = text_right.max(rect.right());
        }
        let mut cursor = None;
        for clipped in &output.shapes {
            if let egui::Shape::LineSegment { points, stroke } = &clipped.shape {
                if points[0].x == points[1].x && stroke.color.a() > 0 {
                    cursor = Some(points[0].x);
                }
            }
        }
        (cursor, text_left, text_right)
    }

    /// The text shows in the field, on the side its alignment puts it, and
    /// isn't clipped away: in the window egui clipped it to the stand-in's
    /// place, which had no width.
    #[test]
    fn the_text_shows_in_the_field() {
        let cases = [
            (Align2::LEFT_CENTER, false),
            (Align2::RIGHT_CENTER, false),
            (Align2::LEFT_TOP, true),
        ];
        for (align, multiline) in cases {
            let ctx = window();
            let mut text = String::from("اجتماع غدا");
            let mut field = Rect::NOTHING;
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(600.0, 200.0))),
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| {
                let edit = if multiline {
                    Field::multiline(&mut text)
                } else {
                    Field::singleline(&mut text)
                };
                field = ui.add(edit.desired_width(300.0).align(align)).rect;
            });
            output.textures_delta.clear();
            let shown = text_shown(&output);
            assert!(!shown.is_empty(), "{align:?}: no text");
            for (rect, clip) in &shown {
                assert!(
                    clip.expand(0.5).contains_rect(*rect),
                    "{align:?}: {rect:?} clipped to {clip:?}"
                );
                assert!(
                    field.contains_rect(*rect),
                    "{align:?}: {rect:?} out of {field:?}"
                );
            }
            let left = shown
                .iter()
                .map(|(r, _)| r.left())
                .fold(f32::INFINITY, f32::min);
            let right = shown
                .iter()
                .map(|(r, _)| r.right())
                .fold(f32::NEG_INFINITY, f32::max);
            // An Arabic line of several sits against the right edge.
            if multiline || align.x() == Align::Max {
                assert!(
                    field.right() - right < 12.0,
                    "{align:?}: {right} in {field:?}"
                );
            } else {
                assert!(left - field.left() < 12.0, "{align:?}: {left} in {field:?}");
            }
        }
    }

    /// A click puts the cursor where it lands, also when it starts and ends
    /// in one frame, as a touchpad tap can: in the window egui then put the
    /// cursor at the start of the text.
    #[test]
    fn a_tap_puts_the_cursor_under_the_pointer() {
        for frames in [1, 2] {
            let mut typing = Typing::new();
            let output = typing.write("اجتماع غدا");
            let shown = text_shown(&output);
            let right = shown
                .iter()
                .map(|(r, _)| r.right())
                .fold(f32::NEG_INFINITY, f32::max);
            // Right of an Arabic line is its start.
            let at = Pos2::new(right + 3.0, shown[0].0.center().y);
            let button = |pressed| Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            };
            if frames == 1 {
                typing.frame(
                    vec![Event::PointerMoved(at), button(true), button(false)],
                    |_| {},
                );
            } else {
                typing.frame(vec![Event::PointerMoved(at), button(true)], |_| {});
                typing.frame(vec![button(false)], |_| {});
            }
            typing.write("x");
            assert_eq!(typing.text, "xاجتماع غدا", "a click in {frames} frames");
        }
    }

    /// The cursor is drawn where typing goes: after typing an Arabic word
    /// at its left end, after Home at its right end.
    #[test]
    fn the_cursor_is_drawn_where_typing_goes() {
        let mut typing = Typing::new();
        let (cursor, left, right) = cursor_and_text(&typing.write("اجتماع"));
        assert!(
            cursor.is_some_and(|x| (x - left).abs() < 1.5),
            "{cursor:?}, text {left}..{right}"
        );
        let (cursor, left, right) = cursor_and_text(&typing.key(Key::Home));
        assert!(
            cursor.is_some_and(|x| (x - right).abs() < 1.5),
            "{cursor:?}, text {left}..{right}"
        );
    }
}
