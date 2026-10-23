//! How an item is described: titles, subtitles and highlighted search terms.

use super::*;

pub(crate) fn looks_like_code(s: &str) -> bool {
    let sample: String = s.chars().take(2000).collect();
    let symbols = sample
        .chars()
        .filter(|c| "{}();=<>[]_/\\".contains(*c))
        .count();
    sample.lines().count() > 1 && symbols * 20 > sample.len().max(1)
}

// ----- time --------------------------------------------------------------------

// Stored times are UTC seconds; `i18n::local` shows each in the user's zone.

/// `text` with every occurrence of the query terms highlighted, cut to `max_rows`.
pub(super) fn highlighted(
    text: &str,
    terms: &[String],
    font: FontId,
    color: Color32,
    max_width: f32,
    max_rows: usize,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    let plain = TextFormat {
        font_id: font.clone(),
        color,
        ..Default::default()
    };
    let hl = TextFormat {
        font_id: font,
        color,
        background: accent().gamma_multiply(0.28),
        ..Default::default()
    };
    let lower = text.to_lowercase();
    // Lower-casing can change byte offsets outside ASCII; skip highlighting then.
    let mut marks: Vec<(usize, usize)> = Vec::new();
    if lower.len() == text.len() {
        for t in terms.iter().filter(|t| !t.is_empty()) {
            let mut from = 0;
            while let Some(i) = lower[from..].find(t.as_str()) {
                marks.push((from + i, from + i + t.len()));
                from += i + t.len();
            }
        }
    }
    marks.sort();
    let mut pos = 0;
    for (a, b) in marks {
        if a < pos || !text.is_char_boundary(a) || !text.is_char_boundary(b) {
            continue;
        }
        job.append(&text[pos..a], 0.0, plain.clone());
        job.append(&text[a..b], 0.0, hl.clone());
        pos = b;
    }
    job.append(&text[pos..], 0.0, plain);
    job.wrap = TextWrapping {
        max_width,
        max_rows,
        break_anywhere: max_rows == 1,
        overflow_character: Some('…'),
    };
    job
}

/// What a row's or card's hover buttons asked for.
#[derive(Clone, Copy)]
pub(super) enum ItemAction {
    Select(u64),
    Copy(u64),
    Delete(u64),
    Pin(u64),
    Open(u64),
}

/// Domain of a URL, e.g. `huggingface.co`.
pub(super) fn domain(url: &str) -> &str {
    let rest = url.trim().split("://").nth(1).unwrap_or(url.trim());
    rest.split(['/', '?', '#'])
        .next()
        .unwrap_or(rest)
        .trim_start_matches("www.")
}

/// Second line of an item row: the most useful detail for its kind.
pub(super) fn subtitle(it: &Item) -> String {
    let detail = subtitle_detail(it);
    if it.tags.is_empty() {
        return detail;
    }
    let tags = it
        .tags
        .iter()
        .map(|t| format!("#{t}"))
        .collect::<Vec<_>>()
        .join(" ");
    if detail.is_empty() {
        tags
    } else {
        format!("{tags}  ·  {detail}")
    }
}

pub(super) fn subtitle_detail(it: &Item) -> String {
    match it.kind {
        Kind::Link => domain(&it.text).to_string(),
        Kind::Image => {
            let mut s = format!("{} × {}", it.width, it.height);
            if let Some(o) = it.ocr.as_deref().filter(|o| !o.trim().is_empty()) {
                let first: String = o.split_whitespace().take(8).collect::<Vec<_>>().join(" ");
                s = format!("{s}  ·  {first}");
            }
            s
        }
        Kind::Files => {
            let n = it.text.lines().count();
            if n == 1 {
                std::path::Path::new(it.text.trim())
                    .parent()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            } else {
                trf("{n} files", &[("n", &n)])
            }
        }
        Kind::Text => {
            if let Some(src) = &it.source {
                return src
                    .file_name()
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap_or_default();
            }
            let mut lines = it.text.lines().map(str::trim).filter(|l| !l.is_empty());
            lines.next();
            lines
                .next()
                .map(|l| l.chars().take(140).collect())
                .unwrap_or_default()
        }
    }
}

pub(super) fn row_title(it: &Item) -> String {
    match it.kind {
        Kind::Link => {
            let t = it.text.trim();
            t.split("://")
                .nth(1)
                .unwrap_or(t)
                .trim_start_matches("www.")
                .to_string()
        }
        Kind::Files => {
            let first = it.text.lines().next().unwrap_or("");
            std::path::Path::new(first)
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_else(|| first.to_string())
        }
        _ => it.title(),
    }
}
