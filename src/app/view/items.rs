//! How an item is described: titles, subtitles and highlighted search terms.

use super::*;
use crate::app::programs::Program;

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
        // Each in its own direction, so a Latin tag keeps its "#" before
        // Arabic text.
        format!("{}  ·  {}", bidi::isolate(&tags), bidi::isolate(&detail))
    }
}

pub(super) fn subtitle_detail(it: &Item) -> String {
    match it.kind {
        Kind::Link => domain(&it.text).to_string(),
        Kind::Text => {
            if let Some(color) = color_in(&it.text) {
                let value = bidi::isolate(&color_other_way(&it.text, color));
                return t!("item-color", value = value);
            }
            if path_in(&it.text).is_some() || it.source.is_some() {
                return row_fact(it).unwrap_or_default();
            }
            let mut lines = it.text.lines().map(str::trim).filter(|l| !l.is_empty());
            lines.next();
            lines
                .next()
                .map(|l| l.chars().take(140).collect())
                .unwrap_or_default()
        }
        Kind::Image | Kind::Files => row_fact(it).unwrap_or_default(),
    }
}

/// The one fact a list row's second line gives after the program: a
/// picture's size, a color written the other usual way, the folder of a
/// file or a path, how many files, or the file a text came from. None for
/// a link, whose title is its address, and for plain text.
pub(super) fn row_fact(it: &Item) -> Option<String> {
    let folder = |path: &std::path::Path| {
        path.parent()
            .map(|p| p.display().to_string())
            .filter(|p| !p.is_empty())
    };
    match it.kind {
        Kind::Link => None,
        // The title is the read text, the file name or "Image": the size
        // is what the second line adds, without repeating the text.
        Kind::Image => Some(format!("{} × {}", it.width, it.height)),
        Kind::Files => match it.text.lines().count() {
            1 => folder(std::path::Path::new(it.text.trim())),
            n => Some(t!("item-files-count", count = n)),
        },
        Kind::Text => {
            if let Some(color) = color_in(&it.text) {
                return Some(color_other_way(&it.text, color));
            }
            if let Some(path) = path_in(&it.text) {
                return folder(path);
            }
            let source = it.source.as_deref()?.file_name()?;
            Some(source.to_string_lossy().into_owned())
        }
    }
}

/// What a screen reader says for an item's row or card: its kind, title,
/// the program it came from, details, whether it is pinned, and when it
/// was copied.
pub(super) fn item_name(it: &Item, program: Option<&Program>) -> String {
    let title = row_title(it);
    // An image with no read text and no file name has the plain title
    // "Image", which its kind already says.
    let first = if it.kind == Kind::Image && title == t!("item-image") {
        it.kind.label()
    } else {
        format!("{}: {}", it.kind.label(), title)
    };
    let mut parts = vec![first];
    if let Some(app) = &it.app {
        parts.push(crate::app::programs::shown_name(app, program));
    }
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
        let name = item_name(&it, None);
        assert_eq!(name.matches("420").count(), 1, "{name}");
        it.ocr = Some("Frame test 007".into());
        let name = item_name(&it, None);
        assert!(
            name.contains("Frame test 007") && name.contains(&size),
            "{name}"
        );
        assert_eq!(name.matches("Frame test 007").count(), 1, "{name}");
    }

    /// A screen reader hears the program an item came from right after
    /// its title: its own name once it has been read, else its file name
    /// tidied.
    #[test]
    fn a_rows_name_says_its_program_after_its_title() {
        let mut it = Item::new(1, Kind::Text, Origin::Clipboard, "Lunch at noon".into());
        it.app = Some("Teams.exe".into());
        let name = item_name(&it, None);
        let (title, program) = (name.find("Lunch at noon"), name.find("Microsoft Teams"));
        assert!(title.is_some() && program > title, "{name}");
        let read = Program {
            name: "Teams (work or school)".into(),
            icon: None,
        };
        let name = item_name(&it, Some(&read));
        let (title, program) = (name.find("Lunch at noon"), name.find("Teams (work"));
        assert!(title.is_some() && program > title, "{name}");
        // Without a program, nothing in its place.
        it.app = None;
        assert!(!item_name(&it, None).contains("Teams"));
    }

    /// The fact a row's second line gives, kind by kind.
    #[test]
    fn a_row_gives_one_fact_for_its_kind() {
        let item = |kind: Kind, text: &str| Item::new(1, kind, Origin::Clipboard, text.into());
        let mut picture = item(Kind::Image, "");
        (picture.width, picture.height) = (480, 320);
        assert_eq!(row_fact(&picture).as_deref(), Some("480 × 320"));
        let color = item(Kind::Text, "#5865F2");
        assert_eq!(row_fact(&color).as_deref(), Some("rgb(88, 101, 242)"));
        // Paths with forward slashes, which every system takes apart.
        let one = item(Kind::Files, "C:/Reports/May.pdf");
        assert_eq!(row_fact(&one).as_deref(), Some("C:/Reports"));
        let several = item(Kind::Files, "C:/Reports/May.pdf\nC:/Reports/June.pdf");
        assert_eq!(row_fact(&several), Some(t!("item-files-count", count = 2)));
        let path = item(Kind::Text, "C:/Users/Public/report.pdf");
        assert_eq!(row_fact(&path).as_deref(), Some("C:/Users/Public"));
        let mut from_file = item(Kind::Text, "Total due on the 15th");
        from_file.source = Some("C:/Notes/invoice.txt".into());
        assert_eq!(row_fact(&from_file).as_deref(), Some("invoice.txt"));
        // A link's title is its address; plain text shows no line of its
        // own there.
        let link = item(Kind::Link, "https://rust-lang.org/learn");
        assert_eq!(row_fact(&link), None);
        assert_eq!(row_fact(&item(Kind::Text, "Lunch\nat noon")), None);
    }
}
