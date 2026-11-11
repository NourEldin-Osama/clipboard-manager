//! Showing right-to-left (Arabic) text correctly.
//!
//! egui shapes Arabic letters (joining, ligatures) but places text runs left to
//! right and wraps in logical order. So for any text containing RTL characters:
//! lines are broken here, and each line is laid out in display order with the
//! Unicode bidi algorithm. Inside a right-to-left run every word gets its own
//! section, in reverse, because egui also splits a run wherever the font changes
//! (an Arabic word, then a space from the Latin font, …) and keeps those pieces
//! in logical order.
//!
//! Text without RTL characters takes egui's normal path.

use std::cell::RefCell;
use std::sync::Arc;

use eframe::egui::text::{LayoutJob, TextFormat, TextWrapping};
use eframe::egui::{Galley, Pos2, Rect, Ui};
use unicode_bidi::{BidiClass, Direction, Level, ParagraphBidiInfo};

use super::view::READING_WIDTH;

use super::*;

thread_local! {
    /// Arabic text drawn in this frame: display order to reading order
    /// (see `read_in_reading_order`).
    static DRAWN: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
}

/// Starts a frame's list of Arabic text drawn.
pub(super) fn forget_drawn() {
    DRAWN.with(|drawn| drawn.borrow_mut().clear());
}

/// Gives screen readers this viewport's Arabic text in reading order.
/// egui names a widget by the text it shows, which is in display order
/// here, so a screen reader read the words reversed. While no screen
/// reader listens, egui builds no accessibility tree and this does nothing.
pub(super) fn read_in_reading_order(ctx: &egui::Context) {
    type Fix = (egui::Id, Option<String>, Option<String>);
    let fixes: Vec<Fix> = ctx.viewport(|v| {
        let Some(state) = &v.this_pass.accesskit_state else {
            return Vec::new();
        };
        DRAWN.with(|drawn| {
            let drawn = drawn.borrow();
            let logical = |text: Option<&str>| text.and_then(|t| drawn.get(t)).cloned();
            state
                .nodes
                .iter()
                // A label's text is its value; other widgets' is their label.
                .map(|(id, node)| (*id, logical(node.label()), logical(node.value())))
                .filter(|(_, label, value)| label.is_some() || value.is_some())
                .collect()
        })
    });
    for (id, label, value) in fixes {
        ctx.accesskit_node_builder(id, |node| {
            if let Some(label) = label {
                node.set_label(label);
            }
            if let Some(value) = value {
                node.set_value(value);
            }
        });
    }
}

fn is_rtl_char(c: char) -> bool {
    matches!(c as u32, 0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF | 0x10800..=0x10FFF | 0x1E800..=0x1EFFF)
}

pub(super) fn has_rtl(s: &str) -> bool {
    s.chars().any(is_rtl_char)
}

/// Direction of a paragraph from its first strong letter; `fallback` when it has none.
pub(super) fn starts_rtl(s: &str, fallback: bool) -> bool {
    match unicode_bidi::get_base_direction(s) {
        Direction::Rtl => true,
        Direction::Ltr => false,
        Direction::Mixed => fallback,
    }
}

/// Unicode's direction isolates: invisible marks around a part of a line
/// that keeps its own direction, left to right or right to left, and the
/// mark that ends the part. egui draws them as nothing.
const LTR_ISOLATE: char = '\u{2066}';
const RTL_ISOLATE: char = '\u{2067}';
const ISOLATE_END: char = '\u{2069}';

/// `text` as a part of a line that keeps its own direction: that of its
/// first letter, or the window's when it has none. A name or a value
/// beside Arabic then doesn't run into the words and numbers around it.
pub(super) fn isolate(text: &str) -> String {
    let start = if starts_rtl(text, crate::i18n::rtl()) {
        RTL_ISOLATE
    } else {
        LTR_ISOLATE
    };
    format!("{start}{text}{ISOLATE_END}")
}

fn mirror(c: char) -> char {
    match c {
        '(' => ')',
        ')' => '(',
        '[' => ']',
        ']' => '[',
        '{' => '}',
        '}' => '{',
        '<' => '>',
        '>' => '<',
        '«' => '»',
        '»' => '«',
        c => c,
    }
}

/// Appends one line (no newlines) in display order. `marks` are highlighted byte ranges.
fn append_line(
    job: &mut LayoutJob,
    line: &str,
    rtl: bool,
    plain: &TextFormat,
    hl: &TextFormat,
    marks: &[(usize, usize)],
) {
    if line.is_empty() {
        return;
    }
    let info = ParagraphBidiInfo::new(line, Some(if rtl { Level::rtl() } else { Level::ltr() }));
    let (levels, runs) = info.visual_runs(0..line.len());
    let is_hl = |at: usize| marks.iter().any(|&(a, b)| at >= a && at < b);
    let fmt = |at: usize| if is_hl(at) { hl } else { plain };
    for run in runs {
        if !levels[run.start].is_rtl() {
            // Left-to-right run: egui handles it; split only where highlighting changes.
            let (mut start, mut cur) = (run.start, is_hl(run.start));
            for (i, _) in line[run.clone()].char_indices() {
                let i = i + run.start;
                if is_hl(i) != cur {
                    job.append(&line[start..i], 0.0, fmt(start).clone());
                    (start, cur) = (i, !cur);
                }
            }
            job.append(&line[start..run.end], 0.0, fmt(start).clone());
            continue;
        }
        // Right-to-left run: words (letters plus combining marks) stay whole and
        // are shaped right to left by egui; everything else is one char at a time,
        // mirrored. The pieces go in reverse.
        let mut pieces: Vec<(usize, String)> = Vec::new();
        let mut word = String::new();
        let mut word_at = run.start;
        for (i, c) in line[run.clone()]
            .char_indices()
            .map(|(i, c)| (i + run.start, c))
        {
            let joins = is_rtl_char(c) || unicode_bidi::bidi_class(c) == BidiClass::NSM;
            if joins {
                if word.is_empty() {
                    word_at = i;
                }
                word.push(c);
            } else {
                if !word.is_empty() {
                    pieces.push((word_at, std::mem::take(&mut word)));
                }
                pieces.push((i, mirror(c).to_string()));
            }
        }
        if !word.is_empty() {
            pieces.push((word_at, word));
        }
        for (at, s) in pieces.into_iter().rev() {
            job.append(&s, 0.0, fmt(at).clone());
        }
    }
}

/// Greedy word wrap of one paragraph into lines that fit `width`.
fn wrap(ui: &Ui, text: &str, font: &FontId, width: f32) -> Vec<String> {
    let measure = |s: &str| {
        ui.fonts_mut(|f| {
            f.layout_no_wrap(s.to_string(), font.clone(), Color32::WHITE)
                .size()
                .x
        })
    };
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_inclusive(' ') {
        let candidate = format!("{line}{word}");
        if line.is_empty() || measure(candidate.trim_end()) <= width {
            line = candidate;
        } else {
            lines.push(std::mem::take(&mut line).trim_end().to_string());
            line = word.to_string();
        }
        // A single word wider than the line is broken by characters.
        while measure(line.trim_end()) > width && line.chars().count() > 1 {
            let mut cut = line.len();
            while cut > 0 && measure(&line[..cut]) > width {
                cut = line[..cut]
                    .char_indices()
                    .last()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
            }
            let cut = cut.max(line.chars().next().map(char::len_utf8).unwrap_or(1));
            lines.push(line[..cut].to_string());
            line = line[cut..].to_string();
        }
    }
    lines.push(line.trim_end().to_string());
    lines
}

/// Laid-out lines of `text`, at most `max_rows` (the last ends in "…" when cut),
/// each with whether it reads right to left.
#[allow(clippy::too_many_arguments)]
/// `rtl` fixes the direction of every paragraph; `None` takes each one's from
/// its first letter.
pub(super) fn lines(
    ui: &Ui,
    text: &str,
    terms: &[String],
    font: FontId,
    color: Color32,
    highlight: Color32,
    width: f32,
    max_rows: usize,
    rtl: Option<bool>,
) -> Vec<(Arc<Galley>, bool)> {
    let plain = TextFormat {
        font_id: font.clone(),
        color,
        ..Default::default()
    };
    // Hits are drawn in the strong text colour, which reads on the tint even
    // where the rest of the line is gray.
    let hl = TextFormat {
        background: highlight,
        color: ui.visuals().strong_text_color(),
        ..plain.clone()
    };
    let mut out: Vec<(String, bool)> = Vec::new();
    for para in text.split('\n') {
        let rtl = rtl.unwrap_or_else(|| starts_rtl(para, crate::i18n::rtl()));
        out.extend(wrap(ui, para, &font, width).into_iter().map(|l| (l, rtl)));
        if out.len() > max_rows {
            break;
        }
    }
    if out.len() > max_rows {
        out.truncate(max_rows.max(1));
        // Shorten the last line until it fits with an ellipsis.
        if let Some((last, _)) = out.last_mut() {
            let fits = |s: &str| {
                ui.fonts_mut(|f| {
                    f.layout_no_wrap(format!("{s}…"), font.clone(), color)
                        .size()
                        .x
                }) <= width
            };
            while !last.is_empty() && !fits(last) {
                last.pop();
            }
            last.push('…');
        }
    }
    out.into_iter()
        .map(|(l, rtl)| {
            let lower = l.to_lowercase();
            let mut marks = Vec::new();
            if lower.len() == l.len() {
                for t in terms.iter().filter(|t| !t.is_empty()) {
                    let mut from = 0;
                    while let Some(i) = lower[from..].find(t.as_str()) {
                        marks.push((from + i, from + i + t.len()));
                        from += i + t.len();
                    }
                }
            }
            let mut job = LayoutJob::default();
            append_line(&mut job, &l, rtl, &plain, &hl, &marks);
            job.wrap = TextWrapping {
                max_width: f32::INFINITY,
                ..Default::default()
            };
            (ui.fonts_mut(|f| f.layout_job(job)), rtl)
        })
        .collect()
}

/// Paints lines from `lines` from the top of `rect`, each against its own edge
/// (right for right-to-left), or all against the right edge when `right` is set.
pub(super) fn paint(
    p: &egui::Painter,
    rect: Rect,
    lines: Vec<(Arc<Galley>, bool)>,
    color: Color32,
    right: Option<bool>,
    gap: f32,
) {
    let mut y = rect.top();
    for (g, rtl) in lines {
        let h = g.size().y;
        let x = if right.unwrap_or(rtl) {
            rect.right() - g.size().x
        } else {
            rect.left()
        };
        p.galley(Pos2::new(x, y), g, color);
        y += h + gap;
    }
}

/// Single-line (or newline-separated) text in display order, for labels and buttons.
pub(super) fn job(text: &str, font: FontId, color: Color32) -> LayoutJob {
    let plain = TextFormat {
        font_id: font,
        color,
        ..Default::default()
    };
    let mut job = LayoutJob::default();
    let ui_rtl = crate::i18n::rtl();
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            job.append("\n", 0.0, plain.clone());
        }
        append_line(
            &mut job,
            line,
            starts_rtl(line, ui_rtl),
            &plain,
            &plain,
            &[],
        );
    }
    if job.text != text {
        DRAWN.with(|drawn| {
            drawn
                .borrow_mut()
                .insert(job.text.clone(), text.to_string())
        });
    }
    job
}

/// A wrapped paragraph as a widget: Arabic breaks into lines in reading order
/// and lines up on the right.
pub(super) fn paragraph(ui: &mut Ui, text: &str, font: FontId, color: Color32) -> egui::Response {
    let width = ui.available_width();
    block(ui, text, font, color, width, 0.0)
}

/// A paragraph as `paragraph` lays it out, its lines evened out: when it
/// takes more than one line in the width it has, it is laid out at the
/// narrowest width that keeps as many lines, so they come out about equally
/// long and the last isn't one short word.
pub(super) fn balanced(ui: &mut Ui, text: &str, font: FontId, color: Color32) -> egui::Response {
    let width = balanced_width(ui, text, &font, ui.available_width());
    block(ui, text, font, color, width, 0.0)
}

/// How many widths `balanced_width` tries at most.
const BALANCE_TRIES: usize = 8;

/// The narrowest width, at most `width`, at which `text` wraps into as many
/// lines as at `width`; `width` itself when each of its paragraphs fits on
/// one line, which measures nothing `lines` doesn't. It halves the range
/// between a width too narrow and one that keeps the lines, and a try that
/// keeps them narrows to its widest line, where they wrap the same. egui
/// keeps what it laid out, so frames after the first lay out nothing new.
fn balanced_width(ui: &Ui, text: &str, font: &FontId, width: f32) -> f32 {
    let measure = |s: &str| {
        ui.fonts_mut(|f| {
            f.layout_no_wrap(s.to_string(), font.clone(), Color32::WHITE)
                .size()
                .x
        })
    };
    if text.split('\n').all(|p| measure(p.trim_end()) <= width) {
        return width;
    }
    // How many lines `text` takes at `w`, and how wide the widest is.
    let wrapped = |w: f32| {
        let lines: Vec<String> = text
            .split('\n')
            .flat_map(|p| wrap(ui, p, font, w))
            .collect();
        let widest = lines.iter().map(|l| measure(l)).fold(0.0, f32::max);
        (lines.len(), widest)
    };
    let (count, mut wide) = wrapped(width);
    // The text shared out over that many lines, less a space at each
    // break: anything narrower takes more lines.
    let whole: f32 = text.split('\n').map(|p| measure(p.trim_end())).sum();
    let mut narrow = whole / count as f32 - font.size;
    for _ in 0..BALANCE_TRIES {
        if wide - narrow <= 1.0 {
            break;
        }
        let middle = (narrow + wide) / 2.0;
        let (lines, widest) = wrapped(middle);
        if lines <= count {
            wide = widest;
        } else {
            narrow = middle;
        }
    }
    wide
}

/// An item's own text to read, as a paragraph: lines at most about 70
/// characters wide, with a little room between them, on the side its first
/// letter reads from, so English content lines up on the left in Arabic too.
pub(super) fn content(ui: &mut Ui, text: &str, font: FontId, color: Color32) -> egui::Response {
    let width = ui.available_width().min(font.size * READING_WIDTH);
    let gap = (font.size * 0.2).round();
    let rtl = starts_rtl(text, crate::i18n::rtl());
    block_on(ui, text, font, color, width, gap, rtl)
}

fn block(
    ui: &mut Ui,
    text: &str,
    font: FontId,
    color: Color32,
    width: f32,
    gap: f32,
) -> egui::Response {
    block_on(ui, text, font, color, width, gap, crate::i18n::rtl())
}

/// A block of lines on the right side when `rtl`, else on the left.
fn block_on(
    ui: &mut Ui,
    text: &str,
    font: FontId,
    color: Color32,
    width: f32,
    gap: f32,
    rtl: bool,
) -> egui::Response {
    let lines = lines(
        ui,
        text,
        &[],
        font,
        color,
        Color32::TRANSPARENT,
        width,
        usize::MAX,
        None,
    );
    let gaps = gap * lines.len().saturating_sub(1) as f32;
    let h: f32 = lines.iter().map(|(g, _)| g.size().y).sum::<f32>() + gaps;
    let (whole, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::hover());
    let rect = if rtl {
        Rect::from_min_max(Pos2::new(whole.right() - width, whole.top()), whole.max)
    } else {
        Rect::from_min_size(whole.min, Vec2::new(width, h))
    };
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, text));
    paint(ui.painter(), rect, lines, color, Some(rtl), gap);
    resp
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::view::size;

    /// The names a screen reader gets for one frame of `draw`.
    fn spoken(draw: impl FnMut(&mut Ui)) -> Vec<String> {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut output = ctx.run_ui(egui::RawInput::default(), draw);
        // The font atlas upload no renderer takes here.
        output.textures_delta.clear();
        let update = output.platform_output.accesskit_update.expect("a tree");
        update
            .nodes
            .iter()
            .flat_map(|(_, node)| [node.label(), node.value()])
            .flatten()
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn screen_readers_get_arabic_in_reading_order() {
        let text = "سجل الحافظة";
        let font = FontId::proportional(size::BODY);
        let shown = job(text, font.clone(), Color32::WHITE).text;
        assert_ne!(shown, text, "drawn in display order");
        let names = spoken(|ui| {
            forget_drawn();
            ui.label(job(text, font.clone(), Color32::WHITE));
            let _ = ui.button(job(text, font.clone(), Color32::WHITE));
            read_in_reading_order(ui.ctx());
        });
        assert_eq!(names.iter().filter(|n| *n == text).count(), 2, "{names:?}");
        assert!(!names.contains(&shown), "{names:?}");
    }

    /// A part set apart keeps its own direction beside Arabic: a colour's
    /// "#" stays before its digits, a Latin tag's before its word, and a
    /// program's name doesn't take in the hint after it. Each line as
    /// drawn, from the left, without the invisible marks.
    #[test]
    fn a_part_keeps_its_own_direction() {
        let plain = TextFormat::default();
        let shown = |text: &str, rtl: bool| -> String {
            let mut job = LayoutJob::default();
            append_line(&mut job, text, rtl, &plain, &plain, &[]);
            let marks = [LTR_ISOLATE, RTL_ISOLATE, ISOLATE_END];
            job.text.chars().filter(|c| !marks.contains(c)).collect()
        };
        let color = format!("لون · {}", isolate("#5865F2"));
        assert_eq!(shown(&color, true), "#5865F2 · لون");
        let tagged = format!("{}  ·  {}", isolate("#work"), isolate("مرحبا بكم"));
        assert_eq!(shown(&tagged, true), "بكم مرحبا  ·  #work");
        assert_eq!(shown(&tagged, false), "#work  ·  بكم مرحبا");
        let skipped = [isolate("من Visual Studio Code"), isolate("sk-…6789")].join(" · ");
        assert_eq!(shown(&skipped, true), "sk-…6789 · Visual Studio Code من");
    }

    /// A window with the app's fonts.
    fn window() -> egui::Context {
        let ctx = egui::Context::default();
        crate::app::view::add_fonts(&ctx);
        // The fonts apply from the next frame; the font atlas uploads no
        // renderer takes here are dropped.
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .textures_delta
            .clear();
        ctx
    }

    /// How wide `text` is on one line in `font`.
    fn width_of(ctx: &egui::Context, text: &str, font: &FontId) -> f32 {
        let mut width = 0.0;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            width = ui.fonts_mut(|f| {
                f.layout_no_wrap(text.into(), font.clone(), Color32::WHITE)
                    .size()
                    .x
            });
        });
        output.textures_delta.clear();
        width
    }

    /// The lines `draw` painted in a column `width` wide, top to bottom.
    fn painted_lines(ctx: &egui::Context, width: f32, mut draw: impl FnMut(&mut Ui)) -> Vec<Rect> {
        let column = Rect::from_min_size(Pos2::new(20.0, 20.0), Vec2::new(width, 400.0));
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.scope_builder(egui::UiBuilder::new().max_rect(column), |ui| draw(ui));
        });
        output.textures_delta.clear();
        let mut lines: Vec<Rect> = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => Some(text.galley.rect.translate(text.pos.to_vec2())),
                _ => None,
            })
            .collect();
        lines.sort_by(|a, b| a.top().total_cmp(&b.top()));
        lines
    }

    /// "Reads the words in screenshots and photos, so search finds them"
    /// in a column where it wraps into two lines, "them" alone on the
    /// second: balanced, it keeps two lines and the second is at least half
    /// as wide as the first. An Arabic line that wraps the same way is
    /// balanced the same way. The lines start on the window's reading side,
    /// and a line that fits on one line is laid out as before.
    fn check_balanced_lines() {
        let ctx = window();
        let font = FontId::proportional(size::SMALL);
        let start = |at: Rect, width: f32| {
            if crate::i18n::rtl() {
                at.right() - (20.0 + width)
            } else {
                at.left() - 20.0
            }
        };
        for (text, last_word) in [
            (
                "Reads the words in screenshots and photos, so search finds them",
                "them",
            ),
            ("يقرأ الكلمات في لقطات الشاشة والصور، ليجدها البحث", "البحث"),
        ] {
            // Just wide enough for all but the last word.
            let rest = text.trim_end_matches(last_word).trim_end();
            let width = width_of(&ctx, rest, &font) + 1.0;
            let plain = painted_lines(&ctx, width, |ui| {
                paragraph(ui, text, font.clone(), Color32::GRAY);
            });
            assert_eq!(plain.len(), 2, "{text}: {plain:?}");
            assert!(
                plain[1].width() < plain[0].width() / 2.0,
                "{text}: {plain:?}"
            );
            let even = painted_lines(&ctx, width, |ui| {
                balanced(ui, text, font.clone(), Color32::GRAY);
            });
            assert_eq!(even.len(), 2, "{text}: {even:?}");
            assert!(even[1].width() >= even[0].width() / 2.0, "{text}: {even:?}");
            for line in &even {
                assert!(start(*line, width).abs() <= 0.5, "{text}: {line:?}");
            }
        }
        let fits = "Keeps running in the tray";
        let plain = painted_lines(&ctx, 300.0, |ui| {
            paragraph(ui, fits, font.clone(), Color32::GRAY);
        });
        let even = painted_lines(&ctx, 300.0, |ui| {
            balanced(ui, fits, font.clone(), Color32::GRAY);
        });
        assert_eq!(plain.len(), 1, "{plain:?}");
        assert_eq!(even, plain);
    }

    #[test]
    fn a_wrapped_line_is_balanced() {
        check_balanced_lines();
    }

    #[test]
    fn a_wrapped_line_is_balanced_right_to_left() {
        crate::i18n::passes_in_arabic(module_path!(), "a_wrapped_line_is_balanced_in_arabic");
    }

    #[test]
    #[ignore = "run in a process of its own by a_wrapped_line_is_balanced_right_to_left"]
    fn a_wrapped_line_is_balanced_in_arabic() {
        if crate::i18n::arabic_run() {
            check_balanced_lines();
        }
    }

    #[test]
    fn a_word_wider_than_the_line_is_broken_wherever_it_comes() {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let font = FontId::proportional(size::SMALL);
            let path = r"(D:\clipboard-manager\target\run\debug\clipboard-manager.part);";
            for text in [path.to_string(), format!("can't be written {path} by hand")] {
                let lines = wrap(ui, &text, &font, 120.0);
                for line in &lines {
                    let width = ui.fonts_mut(|f| {
                        f.layout_no_wrap(line.clone(), font.clone(), Color32::WHITE)
                            .size()
                            .x
                    });
                    assert!(width <= 120.0, "{line:?} is {width} wide, in {lines:?}");
                }
                assert_eq!(lines.concat().replace(' ', ""), text.replace(' ', ""));
            }
        });
        // The font atlas upload no renderer takes here.
        output.textures_delta.clear();
    }
}
