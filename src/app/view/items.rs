//! How an item is described: titles, subtitles and highlighted search terms.

use super::*;

pub(crate) fn looks_like_code(s: &str) -> bool {
    let sample: String = s.chars().take(2000).collect();
    let symbols = sample
        .chars()
        .filter(|c| "{}();=<>[]_/\\".contains(*c))
        .count();
    looks_like_sql(&sample) || (sample.lines().count() > 1 && symbols * 20 > sample.len().max(1))
}

/// An SQL statement, often one line: a statement word in capitals first and
/// one of its clauses in capitals later, so a sentence that starts with
/// "Select" doesn't count.
fn looks_like_sql(sample: &str) -> bool {
    let text = sample.trim_start();
    let statements = [
        "SELECT ",
        "INSERT INTO ",
        "UPDATE ",
        "DELETE FROM ",
        "CREATE TABLE ",
        "WITH ",
    ];
    let clauses = [" FROM ", " WHERE ", " SET ", " VALUES", " JOIN ", " AS ("];
    statements.iter().any(|s| text.starts_with(s)) && clauses.iter().any(|c| text.contains(c))
}

/// A text item that looks like code.
pub(super) fn is_code(it: &Item) -> bool {
    it.kind == Kind::Text && looks_like_code(&it.text.chars().take(600).collect::<String>())
}

/// The icon for an item: code gets its own, other items their kind's.
pub(super) fn item_icon(it: &Item) -> Icon {
    if is_code(it) {
        Icon::Code
    } else {
        icons::kind(it.kind)
    }
}

// ----- time --------------------------------------------------------------------

// Stored times are UTC seconds; `i18n::local` shows each in the user's zone.

/// `text` with every occurrence of the query terms highlighted, cut to
/// `max_rows`. Hits are drawn in `hit`, the strong text colour, which reads on
/// the accent tint even where the rest of the line is gray.
pub(super) fn highlighted(
    text: &str,
    terms: &[String],
    font: FontId,
    color: Color32,
    // The found words' color and background.
    hit: (Color32, Color32),
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
        color: hit.0,
        background: hit.1,
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

/// Domain of a URL, e.g. `huggingface.co`.
pub(super) fn domain(url: &str) -> &str {
    let rest = url.trim().split("://").nth(1).unwrap_or(url.trim());
    rest.split(['/', '?', '#'])
        .next()
        .unwrap_or(rest)
        .trim_start_matches("www.")
}

/// The colour a text is, when the whole text is one: `#rgb`, `#rrggbb`,
/// `#rrggbbaa`, `rgb()`, `rgba()`, `hsl()` or `hsla()`.
pub(super) fn color_in(text: &str) -> Option<Color32> {
    let text = text.trim();
    if text.len() > 40 || text.contains('\n') {
        return None;
    }
    if let Some(hex) = text.strip_prefix('#') {
        let digit = |i: usize| u8::from_str_radix(hex.get(i..i + 1)?, 16).ok();
        let pair = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
        return match hex.len() {
            3 => {
                let (r, g, b) = (digit(0)? * 17, digit(1)? * 17, digit(2)? * 17);
                Some(Color32::from_rgb(r, g, b)) // copied color
            }
            6 => Some(Color32::from_rgb(pair(0)?, pair(2)?, pair(4)?)), // copied color
            8 => {
                let (r, g, b, a) = (pair(0)?, pair(2)?, pair(4)?, pair(6)?);
                Some(Color32::from_rgba_unmultiplied(r, g, b, a)) // copied color
            }
            _ => None,
        };
    }
    let lower = text.to_ascii_lowercase();
    let (name, rest) = lower.split_once('(')?;
    let inside = rest.strip_suffix(')')?;
    let parts: Vec<&str> = inside
        .split([',', ' ', '/'])
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() < 3 {
        return None;
    }
    let number = |part: &str| part.trim_end_matches('%').parse::<f32>().ok();
    let (a, b, c) = (number(parts[0])?, number(parts[1])?, number(parts[2])?);
    match name.trim() {
        "rgb" | "rgba" if [a, b, c].iter().all(|v| (0.0..=255.0).contains(v)) => {
            Some(Color32::from_rgb(a as u8, b as u8, c as u8)) // copied color
        }
        "hsl" | "hsla" if (0.0..=100.0).contains(&b) && (0.0..=100.0).contains(&c) => {
            let (r, g, bl) = hsl_to_rgb(a, b / 100.0, c / 100.0);
            Some(Color32::from_rgb(r, g, bl)) // copied color
        }
        _ => None,
    }
}

fn hsl_to_rgb(hue: f32, saturation: f32, lightness: f32) -> (u8, u8, u8) {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let sector = hue.rem_euclid(360.0) / 60.0;
    let x = chroma * (1.0 - (sector % 2.0 - 1.0).abs());
    let (r, g, b) = match sector as u32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = lightness - chroma / 2.0;
    let channel = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    (channel(r), channel(g), channel(b))
}

/// The colour written the other usual way: a hex colour as `rgb()`, any
/// other as hex.
pub(super) fn color_other_way(text: &str, color: Color32) -> String {
    if text.trim().starts_with('#') {
        format!("rgb({}, {}, {})", color.r(), color.g(), color.b())
    } else {
        format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b())
    }
}

/// The path, when the whole text is one path to a file or folder: on
/// Windows `C:\...` or `\\server\share\...`, elsewhere `/...` too.
/// The file an item was added from, or the file or folder its text names.
pub(super) fn place_of(it: &Item) -> Option<&std::path::Path> {
    it.source
        .as_deref()
        .or_else(|| (it.kind == Kind::Text).then(|| path_in(&it.text)).flatten())
}

pub(super) fn path_in(text: &str) -> Option<&std::path::Path> {
    let text = text.trim().trim_matches('"');
    if text.len() < 3 || text.contains(['\n', '<', '>', '|', '*', '?']) {
        return None;
    }
    let bytes = text.as_bytes();
    let drive =
        bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && matches!(bytes[2], b'\\' | b'/');
    let share = text.starts_with("\\\\") && text[2..].contains('\\');
    let unix = !cfg!(windows) && text.starts_with('/') && text[1..].contains('/');
    (drive || share || unix).then(|| std::path::Path::new(text))
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
        // The title is the read text, the file name or "Image": the size
        // is what the second line adds, without repeating the text.
        Kind::Image => format!("{} × {}", it.width, it.height),
        Kind::Files => {
            let n = it.text.lines().count();
            if n == 1 {
                std::path::Path::new(it.text.trim())
                    .parent()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            } else {
                t!("item-files-count", count = n)
            }
        }
        Kind::Text => {
            if let Some(color) = color_in(&it.text) {
                return t!("item-color", value = color_other_way(&it.text, color));
            }
            if let Some(path) = path_in(&it.text) {
                return path
                    .parent()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
            }
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

/// What a screen reader says for an item's row or card: its kind, title,
/// details, whether it is pinned, and when it was copied.
pub(super) fn item_name(it: &Item) -> String {
    let title = row_title(it);
    // An image with no read text and no file name has the plain title
    // "Image", which its kind already says.
    let first = if it.kind == Kind::Image && title == t!("item-image") {
        it.kind.label()
    } else {
        format!("{}: {}", it.kind.label(), title)
    };
    let mut parts = vec![first];
    let detail = subtitle_detail(it);
    if !detail.trim().is_empty() {
        parts.push(detail);
    }
    if it.pinned {
        parts.push(t!("details-pinned"));
    }
    parts.push(ago(it.created));
    parts.join(", ")
}

pub(super) fn row_title(it: &Item) -> String {
    match it.kind {
        Kind::Link => {
            let t = it.text.trim();
            let shown = t.split("://").nth(1).unwrap_or(t);
            crate::net::readable_url(shown.trim_start_matches("www.")).into_owned()
        }
        Kind::Files => {
            let first = it.text.lines().next().unwrap_or("");
            std::path::Path::new(first)
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_else(|| first.to_string())
        }
        Kind::Text => match path_in(&it.text) {
            Some(path) => path
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string()),
            None => it.title(),
        },
        Kind::Image => it.title(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Origin;

    #[test]
    fn sql_reads_as_code_and_a_sentence_does_not() {
        assert!(looks_like_code(
            "SELECT name, total FROM orders WHERE total > 1000 ORDER BY total DESC;"
        ));
        assert!(looks_like_code("UPDATE users SET name = 'x' WHERE id = 1"));
        assert!(!looks_like_code("Select the items from the list"));
        assert!(!looks_like_code("SELECT YOUR SEAT"));
    }

    #[test]
    fn colours_are_read_in_the_usual_ways() {
        let blurple = Color32::from_rgb(88, 101, 242);
        assert_eq!(color_in("#5865F2"), Some(blurple));
        assert_eq!(color_in(" #fff "), Some(Color32::WHITE));
        assert_eq!(color_in("rgb(88, 101, 242)"), Some(blurple));
        assert_eq!(
            color_in("hsl(0, 100%, 50%)"),
            Some(Color32::from_rgb(255, 0, 0))
        );
        assert_eq!(color_in("#12345"), None);
        assert_eq!(color_in("rgb(300, 0, 0)"), None);
        assert_eq!(color_in("Meeting at #5"), None);
        assert_eq!(color_other_way("#5865F2", blurple), "rgb(88, 101, 242)");
        assert_eq!(color_other_way("rgb(88, 101, 242)", blurple), "#5865F2");
    }

    #[test]
    fn a_path_counts_only_when_it_is_the_whole_text() {
        assert!(path_in(r"C:\Users\Public\report.pdf").is_some());
        assert!(path_in(r#""C:\Program Files\App""#).is_some());
        assert!(path_in(r"\\server\share\notes.txt").is_some());
        assert!(path_in(r"see C:\file.txt").is_none());
        assert!(path_in("C:\\a\nC:\\b").is_none());
        assert!(path_in("https://example.com/a/b").is_none());
    }

    #[test]
    fn an_image_card_says_its_size_once() {
        let mut it = Item::new(1, Kind::Image, Origin::Clipboard, "".into());
        it.width = 420;
        it.height = 90;
        let size = format!("{} × {}", it.width, it.height);
        let name = item_name(&it);
        assert_eq!(name.matches("420").count(), 1, "{name}");
        it.ocr = Some("Frame test 007".into());
        let name = item_name(&it);
        assert!(
            name.contains("Frame test 007") && name.contains(&size),
            "{name}"
        );
        assert_eq!(name.matches("Frame test 007").count(), 1, "{name}");
    }
}
