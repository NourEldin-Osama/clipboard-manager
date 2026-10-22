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

use std::sync::Arc;

use eframe::egui::text::{LayoutJob, TextFormat, TextWrapping};
use eframe::egui::{Galley, Pos2, Rect, Ui};
use unicode_bidi::{BidiClass, Direction, Level, ParagraphBidiInfo};

use super::*;

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
        } else {
            lines.push(std::mem::take(&mut line).trim_end().to_string());
            line = word.to_string();
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
    let hl = TextFormat {
        background: highlight,
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
        y += h;
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
    job
}

/// A wrapped paragraph as a widget: Arabic breaks into lines in reading order
/// and lines up on the right.
pub(super) fn paragraph(ui: &mut Ui, text: &str, font: FontId, color: Color32) -> egui::Response {
    let width = ui.available_width();
    let rtl = crate::i18n::rtl();
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
    let h: f32 = lines.iter().map(|(g, _)| g.size().y).sum();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, h), Sense::hover());
    paint(ui.painter(), rect, lines, color, Some(rtl));
    resp
}
