//! Markdown as plain text, for copying an item out without its marks
//! (Settings, Clipboard, Advanced): headings, bold, italic and crossed-out
//! text, inline code and code fences, links and pictures, list bullets,
//! quotes and lines across the page, each kind on its own. Tables stay as
//! they are.

use serde::{Deserialize, Serialize};

/// The kinds of marks `plain` removes: a kind that is true goes, one that
/// is false stays as it is. All of them go by default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Marks {
    /// The `#` before a heading and after it, and a line of `=` under one.
    pub headings: bool,
    /// `**`, `__`, `*` and `_` around words.
    pub bold_and_italic: bool,
    /// `~~` around words.
    pub crossed_out: bool,
    /// The backticks around code, and the lines that open and close a
    /// block of code.
    pub code: bool,
    /// `[text](address)`, `![text](address)`, `[text][1]` and `<address>`.
    pub links_and_images: bool,
    /// The `-`, `*` or `+` before a list's item, and a task's box.
    pub list_bullets: bool,
    /// The `>` before a quoted line.
    pub quotes: bool,
    /// Lines across the page, such as `---` or `* * *`.
    pub divider_lines: bool,
}

impl Default for Marks {
    fn default() -> Self {
        Self {
            headings: true,
            bold_and_italic: true,
            crossed_out: true,
            code: true,
            links_and_images: true,
            list_bullets: true,
            quotes: true,
            divider_lines: true,
        }
    }
}

impl Marks {
    /// Whether any kind of mark goes.
    fn any(&self) -> bool {
        let Self {
            headings,
            bold_and_italic,
            crossed_out,
            code,
            links_and_images,
            list_bullets,
            quotes,
            divider_lines,
        } = *self;
        headings
            || bold_and_italic
            || crossed_out
            || code
            || links_and_images
            || list_bullets
            || quotes
            || divider_lines
    }

    /// Whether the marks of the kind `mark` belongs to in a line's text go.
    /// A backslash before a mark keeps it from acting, so the backslash
    /// goes with them; before other punctuation, it goes while any kind
    /// goes.
    fn go_with(&self, mark: char) -> bool {
        match mark {
            '#' => self.headings,
            '*' | '_' => self.bold_and_italic,
            '~' => self.crossed_out,
            '`' => self.code,
            '[' | ']' | '(' | ')' | '!' | '<' => self.links_and_images,
            '-' | '+' => self.list_bullets,
            '>' => self.quotes,
            _ => self.any(),
        }
    }
}

/// `text` without the Markdown marks of the kinds `marks` removes: what a
/// reader of the formatted text sees. Marks inside code always stay, and
/// so do words between underscores such as `__init__` and sums such as
/// `2*3*4`. Line ends stay as `text` has them.
pub fn plain(text: &str, marks: &Marks) -> String {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines = Vec::new();
    // The run of backticks or tildes that opened the code the lines are in.
    let mut fence: Option<String> = None;
    for line in text.lines() {
        let start = line.trim_start();
        if let Some(open) = &fence {
            let closing = closes(start, open);
            if closing {
                fence = None;
            }
            // The code's lines stay; its fences go with the code's marks.
            if !closing || !marks.code {
                lines.push(line.to_string());
            }
            continue;
        }
        if let Some(open) = fence_opening(start) {
            fence = Some(open);
            if !marks.code {
                lines.push(line.to_string());
            }
            continue;
        }
        if let Some(mark) = line_across(start) {
            // A line of `=` only ever underlines a heading.
            let goes = if mark == '=' {
                marks.headings
            } else {
                marks.divider_lines
            };
            if !goes {
                lines.push(line.to_string());
            }
            continue;
        }
        lines.push(block_line(line, marks));
    }
    let mut out = lines.join(newline);
    if text.ends_with('\n') {
        out.push_str(newline);
    }
    out
}

/// The run of three or more backticks or tildes a code fence opens with.
fn fence_opening(start: &str) -> Option<String> {
    let first = start.chars().next().filter(|c| matches!(c, '`' | '~'))?;
    let run = start.chars().take_while(|&c| c == first).count();
    (run >= 3).then(|| first.to_string().repeat(run))
}

/// Whether a line closes the code fence opened with `open`: as long a run
/// of the same mark, or longer, and nothing after it.
fn closes(start: &str, open: &str) -> bool {
    let Some(mark) = open.chars().next() else {
        return false;
    };
    let run = start.chars().take_while(|&c| c == mark).count();
    run >= open.chars().count() && start[run * mark.len_utf8()..].trim().is_empty()
}

/// The mark of a line across the page, or under a heading: three or more
/// of the same mark and nothing else, such as `---` or `* * *`.
fn line_across(start: &str) -> Option<char> {
    let marks: Vec<char> = start.chars().filter(|c| !c.is_whitespace()).collect();
    let across = marks.len() >= 3
        && matches!(marks[0], '-' | '*' | '_' | '=')
        && marks.iter().all(|&c| c == marks[0]);
    across.then(|| marks[0])
}

/// A line without the marks `marks` removes: its quote marks, heading
/// marks or list bullet, and the marks in its text; its indent stays.
fn block_line(line: &str, marks: &Marks) -> String {
    let start = line.trim_start();
    let indent = &line[..line.len() - start.len()];
    let mut rest = start;
    // One quote mark for each level.
    while let Some(after) = rest.strip_prefix('>') {
        rest = after.strip_prefix(' ').unwrap_or(after);
    }
    let quotes = if marks.quotes {
        ""
    } else {
        &start[..start.len() - rest.len()]
    };
    if marks.headings {
        rest = heading(rest).unwrap_or(rest);
    }
    if marks.list_bullets {
        rest = bullet(rest).unwrap_or(rest);
    }
    format!("{indent}{quotes}{}", inline(rest, marks))
}

/// A heading's text: after one to six `#` and a space, without a closing
/// run of `#`. None for other lines, such as a `#hashtag`.
fn heading(rest: &str) -> Option<&str> {
    let hashes = rest.chars().take_while(|&c| c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let after = &rest[hashes..];
    if !(after.is_empty() || after.starts_with([' ', '\t'])) {
        return None;
    }
    let title = after.trim();
    let open = title.trim_end_matches('#');
    if open.len() < title.len() && (open.is_empty() || open.ends_with([' ', '\t'])) {
        return Some(open.trim_end());
    }
    Some(title)
}

/// A list item's text after its bullet (`-`, `*` or `+` and a space), and
/// after a task's box. None for other lines; numbered items keep their
/// numbers.
fn bullet(rest: &str) -> Option<&str> {
    let mark = rest
        .chars()
        .next()
        .filter(|c| matches!(c, '-' | '*' | '+'))?;
    let after = &rest[mark.len_utf8()..];
    let item = after
        .strip_prefix(' ')
        .or_else(|| after.strip_prefix('\t'))?
        .trim_start();
    Some(
        ["[ ] ", "[x] ", "[X] "]
            .iter()
            .find_map(|task| item.strip_prefix(task))
            .unwrap_or(item),
    )
}

/// A line's text without the inline marks `marks` removes: code spans,
/// pictures, links, bold, italic and crossed-out text, and the backslashes
/// before marks. Code spans that stay are kept whole, marks and all.
fn inline(text: &str, marks: &Marks) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' if chars.get(i + 1).is_some_and(char::is_ascii_punctuation) => {
                let escaped = chars[i + 1];
                if !marks.go_with(escaped) {
                    out.push('\\');
                }
                out.push(escaped);
                i += 2;
            }
            '`' => {
                let run = run_at(&chars, i, '`');
                match run_from(&chars, i + run, '`', run) {
                    Some(close) => {
                        if marks.code {
                            let code: String = chars[i + run..close].iter().collect();
                            out.push_str(code_text(&code));
                        } else {
                            out.extend(&chars[i..close + run]);
                        }
                        i = close + run;
                    }
                    None => {
                        out.extend(&chars[i..i + run]);
                        i += run;
                    }
                }
            }
            '!' if chars.get(i + 1) == Some(&'[') => match link(&chars, i + 1) {
                Some(found) => {
                    out.push_str(&linked(&chars, i, &found, marks));
                    i = found.end;
                }
                None => {
                    out.push('!');
                    i += 1;
                }
            },
            '[' => match link(&chars, i) {
                Some(found) => {
                    out.push_str(&linked(&chars, i, &found, marks));
                    i = found.end;
                }
                None => {
                    out.push('[');
                    i += 1;
                }
            },
            '<' if marks.links_and_images => match address(&chars, i) {
                Some((address, end)) => {
                    out.push_str(&address);
                    i = end;
                }
                None => {
                    out.push('<');
                    i += 1;
                }
            },
            '*' | '_' | '~' if marks.go_with(c) => {
                let run = run_at(&chars, i, c);
                match emphasis_end(&chars, i, c, run) {
                    Some(close) => {
                        let inside: String = chars[i + run..close].iter().collect();
                        out.push_str(&inline(&inside, marks));
                        i = close + run;
                    }
                    None => {
                        out.extend(&chars[i..i + run]);
                        i += run;
                    }
                }
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// How many `mark`s in a row start at `at`.
fn run_at(chars: &[char], at: usize, mark: char) -> usize {
    chars[at..].iter().take_while(|&&c| c == mark).count()
}

/// Where the next run of exactly `len` `mark`s starts, from `from` on.
fn run_from(chars: &[char], from: usize, mark: char, len: usize) -> Option<usize> {
    let mut j = from;
    while j < chars.len() {
        if chars[j] == mark {
            let run = run_at(chars, j, mark);
            if run == len {
                return Some(j);
            }
            j += run;
        } else {
            j += 1;
        }
    }
    None
}

/// A code span's text: one space at each end goes when both are there.
fn code_text(code: &str) -> &str {
    match code.strip_prefix(' ').and_then(|c| c.strip_suffix(' ')) {
        Some(inner) if !inner.trim().is_empty() => inner,
        _ => code,
    }
}

/// A link or a picture found in a line's text.
struct Link {
    /// Where the text it shows is: between its brackets.
    text: std::ops::Range<usize>,
    /// Where it goes; None for a reference such as `[text][1]`.
    address: Option<String>,
    /// Where it ends: past its last bracket.
    end: usize,
}

/// A link or a picture starting at the `[` at `at`. A `[` that starts
/// neither, such as a task's `[ ]`, gives None.
fn link(chars: &[char], at: usize) -> Option<Link> {
    let label_end = closing(chars, at, '[', ']')?;
    let text = at + 1..label_end;
    match chars.get(label_end + 1) {
        Some('(') => {
            let target_end = closing(chars, label_end + 1, '(', ')')?;
            let target: String = chars[label_end + 2..target_end].iter().collect();
            let address = target.split_whitespace().next().unwrap_or("").to_string();
            Some(Link {
                text,
                address: Some(address),
                end: target_end + 1,
            })
        }
        Some('[') => {
            let reference_end = closing(chars, label_end + 1, '[', ']')?;
            Some(Link {
                text,
                address: None,
                end: reference_end + 1,
            })
        }
        _ => None,
    }
}

/// The link or picture `found` from `from`, its `[` or the `!` before it:
/// the text it shows without its marks, or its address when it shows none.
/// While links stay, all of it, only its text without its marks.
fn linked(chars: &[char], from: usize, found: &Link, marks: &Marks) -> String {
    let label: String = chars[found.text.clone()].iter().collect();
    let shown = inline(&label, marks);
    if !marks.links_and_images {
        let before: String = chars[from..found.text.start].iter().collect();
        let after: String = chars[found.text.end..found.end].iter().collect();
        return format!("{before}{shown}{after}");
    }
    match &found.address {
        Some(address) if label.trim().is_empty() => address.clone(),
        _ => shown,
    }
}

/// Where the `close` that matches the `open` at `at` is, past any pairs
/// inside and backslashed marks.
fn closing(chars: &[char], at: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0;
    let mut j = at;
    while j < chars.len() {
        match chars[j] {
            '\\' => j += 1,
            c if c == open => depth += 1,
            c if c == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(j);
                }
            }
            _ => {}
        }
        j += 1;
    }
    None
}

/// An address in angle brackets starting at `at`, such as
/// `<https://example.com>` or `<name@example.com>`, and where it ends.
fn address(chars: &[char], at: usize) -> Option<(String, usize)> {
    let end = at + chars[at..].iter().position(|&c| c == '>')?;
    let inside: String = chars[at + 1..end].iter().collect();
    let web = ["http://", "https://", "ftp://", "mailto:"]
        .iter()
        .any(|scheme| inside.to_ascii_lowercase().starts_with(scheme));
    let mail = inside.contains('@') && inside.contains('.');
    let one_word = !inside.is_empty() && !inside.chars().any(char::is_whitespace);
    (one_word && (web || mail)).then_some((inside, end + 1))
}

/// Where the run of `run` `mark`s that ends the bold, italic or crossed-out
/// text opened at `at` starts. Marks count at the edges of words only, so
/// `snake_case` and `2*3*4` stay; text between underscores needs a space
/// in it, so `__init__` stays too. Crossed-out text takes two tildes.
fn emphasis_end(chars: &[char], at: usize, mark: char, run: usize) -> Option<usize> {
    let allowed = match mark {
        '~' => run == 2,
        _ => run <= 3,
    };
    let after_open = *chars.get(at + run)?;
    let opens = at
        .checked_sub(1)
        .is_none_or(|b| !chars[b].is_alphanumeric());
    if !allowed || !opens || after_open.is_whitespace() {
        return None;
    }
    let mut j = at + run + 1;
    while j < chars.len() {
        if chars[j] != mark {
            j += 1;
            continue;
        }
        let found = run_at(chars, j, mark);
        let closes = found == run
            && !chars[j - 1].is_whitespace()
            && chars.get(j + run).is_none_or(|c| !c.is_alphanumeric());
        if closes {
            let inside = &chars[at + run..j];
            if mark != '_' || inside.iter().any(|c| c.is_whitespace()) {
                return Some(j);
            }
            return None;
        }
        j += found;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::Marks;

    /// `text` without the marks of every kind, as by default.
    fn plain(text: &str) -> String {
        super::plain(text, &Marks::default())
    }

    /// No kind of mark goes.
    const NONE: Marks = Marks {
        headings: false,
        bold_and_italic: false,
        crossed_out: false,
        code: false,
        links_and_images: false,
        list_bullets: false,
        quotes: false,
        divider_lines: false,
    };

    /// A text with marks of every kind, and code with marks inside it.
    const EVERY_KIND: &str = "# Title ##\n\
        Some **bold**, _an italic_, ~~gone~~ and `a_b*c*`.\n\
        See [the *docs*](https://x.com/a_b*c*) and ![a chart](chart.png).\n\
        - [ ] a task\n\
        > quoted\n\
        ---\n\
        Under\n\
        ===\n\
        ```rust\n\
        let *x* = 1;\n\
        ```";

    /// Each kind of mark goes on its own, and the others stay.
    #[test]
    fn each_kind_of_mark_goes_on_its_own() {
        // The one kind that goes, and the text left.
        type Case = (fn(&mut Marks), &'static str);
        let cases: [Case; 8] = [
            (
                |marks| marks.headings = true,
                "Title\n\
                 Some **bold**, _an italic_, ~~gone~~ and `a_b*c*`.\n\
                 See [the *docs*](https://x.com/a_b*c*) and ![a chart](chart.png).\n\
                 - [ ] a task\n> quoted\n---\nUnder\n```rust\nlet *x* = 1;\n```",
            ),
            (
                |marks| marks.bold_and_italic = true,
                "# Title ##\n\
                 Some bold, an italic, ~~gone~~ and `a_b*c*`.\n\
                 See [the docs](https://x.com/a_b*c*) and ![a chart](chart.png).\n\
                 - [ ] a task\n> quoted\n---\nUnder\n===\n```rust\nlet *x* = 1;\n```",
            ),
            (
                |marks| marks.crossed_out = true,
                "# Title ##\n\
                 Some **bold**, _an italic_, gone and `a_b*c*`.\n\
                 See [the *docs*](https://x.com/a_b*c*) and ![a chart](chart.png).\n\
                 - [ ] a task\n> quoted\n---\nUnder\n===\n```rust\nlet *x* = 1;\n```",
            ),
            (
                |marks| marks.code = true,
                "# Title ##\n\
                 Some **bold**, _an italic_, ~~gone~~ and a_b*c*.\n\
                 See [the *docs*](https://x.com/a_b*c*) and ![a chart](chart.png).\n\
                 - [ ] a task\n> quoted\n---\nUnder\n===\nlet *x* = 1;",
            ),
            (
                |marks| marks.links_and_images = true,
                "# Title ##\n\
                 Some **bold**, _an italic_, ~~gone~~ and `a_b*c*`.\n\
                 See the *docs* and a chart.\n\
                 - [ ] a task\n> quoted\n---\nUnder\n===\n```rust\nlet *x* = 1;\n```",
            ),
            (
                |marks| marks.list_bullets = true,
                "# Title ##\n\
                 Some **bold**, _an italic_, ~~gone~~ and `a_b*c*`.\n\
                 See [the *docs*](https://x.com/a_b*c*) and ![a chart](chart.png).\n\
                 a task\n> quoted\n---\nUnder\n===\n```rust\nlet *x* = 1;\n```",
            ),
            (
                |marks| marks.quotes = true,
                "# Title ##\n\
                 Some **bold**, _an italic_, ~~gone~~ and `a_b*c*`.\n\
                 See [the *docs*](https://x.com/a_b*c*) and ![a chart](chart.png).\n\
                 - [ ] a task\nquoted\n---\nUnder\n===\n```rust\nlet *x* = 1;\n```",
            ),
            (
                |marks| marks.divider_lines = true,
                "# Title ##\n\
                 Some **bold**, _an italic_, ~~gone~~ and `a_b*c*`.\n\
                 See [the *docs*](https://x.com/a_b*c*) and ![a chart](chart.png).\n\
                 - [ ] a task\n> quoted\nUnder\n===\n```rust\nlet *x* = 1;\n```",
            ),
        ];
        for (only, want) in cases {
            let mut marks = NONE;
            only(&mut marks);
            assert_eq!(super::plain(EVERY_KIND, &marks), want, "{marks:?}");
        }
        assert_eq!(super::plain(EVERY_KIND, &NONE), EVERY_KIND, "none go");
        assert_eq!(
            plain(EVERY_KIND),
            "Title\nSome bold, an italic, gone and a_b*c*.\nSee the docs and a chart.\n\
             a task\nquoted\nUnder\nlet *x* = 1;"
        );
    }

    /// A kind that stays keeps the backslashes before its marks, and a
    /// quote, a heading or a bullet that stays keeps the line's other
    /// marks going.
    #[test]
    fn marks_that_stay_keep_their_backslashes() {
        let only_quotes = Marks {
            quotes: true,
            ..NONE
        };
        assert_eq!(super::plain(r"\*a\* \> b", &only_quotes), r"\*a\* > b");
        let all_but_quotes = Marks {
            quotes: false,
            ..Marks::default()
        };
        assert_eq!(
            super::plain("> # A **title**\n> - an *item*", &all_but_quotes),
            "> A title\n> an item"
        );
        let all_but_headings = Marks {
            headings: false,
            ..Marks::default()
        };
        assert_eq!(
            super::plain("## A **part** ##", &all_but_headings),
            "## A part ##"
        );
        let all_but_bullets = Marks {
            list_bullets: false,
            ..Marks::default()
        };
        assert_eq!(
            super::plain("* an *item*\n- [x] done", &all_but_bullets),
            "* an item\n- [x] done"
        );
    }

    /// Settings saved before the kinds could be chosen remove all of them.
    #[test]
    fn every_kind_goes_unless_the_settings_say_otherwise() {
        let saved: Marks = serde_json::from_str(r#"{"quotes": false}"#).unwrap();
        assert_eq!(
            saved,
            Marks {
                quotes: false,
                ..Marks::default()
            }
        );
        assert!(Marks::default().any() && !NONE.any());
    }

    #[test]
    fn marks_around_words_go() {
        assert_eq!(plain("**bold** and *italic*"), "bold and italic");
        assert_eq!(
            plain("__bold words__ and _some italic_"),
            "bold words and some italic"
        );
        assert_eq!(plain("***both*** and ~~gone~~"), "both and gone");
        assert_eq!(
            plain("**bold *and italic* inside**"),
            "bold and italic inside"
        );
        assert_eq!(
            plain("`code` and ``a `tick` in code``"),
            "code and a `tick` in code"
        );
        assert_eq!(plain(r"a \*star\* stays"), "a *star* stays");
    }

    #[test]
    fn links_and_pictures_keep_their_text() {
        assert_eq!(
            plain("See [the docs](https://example.com/a_b(c)) now"),
            "See the docs now"
        );
        assert_eq!(plain("![A chart](chart.png \"Sales\")"), "A chart");
        assert_eq!(plain("[](https://example.com)"), "https://example.com");
        assert_eq!(
            plain("[a reference][1] and <https://example.com>"),
            "a reference and https://example.com"
        );
        assert_eq!(
            plain("Mail <name@example.com>, not <b>"),
            "Mail name@example.com, not <b>"
        );
    }

    #[test]
    fn lines_lose_their_block_marks() {
        let text = "# Title\n\n## A part ##\n> quoted\n> > twice\n- one\n* two\n  + nested\n- [x] done\n1. kept\n\n---\nend";
        let want = "Title\n\nA part\nquoted\ntwice\none\ntwo\n  nested\ndone\n1. kept\n\nend";
        assert_eq!(plain(text), want);
    }

    #[test]
    fn marks_inside_code_stay() {
        let text = "Run:\n```python\ndef __init__(self, **kwargs):\n    return a * b\n```\ndone";
        assert_eq!(
            plain(text),
            "Run:\ndef __init__(self, **kwargs):\n    return a * b\ndone"
        );
        assert_eq!(plain("~~~\n# not a heading\n~~~"), "# not a heading");
    }

    #[test]
    fn plain_text_stays_as_it_is() {
        for text in [
            "snake_case_name and __init__",
            "2*3*4 = 24, and 2 * 3 * 4 too",
            r"C:\Users\Public\report.pdf",
            "https://example.com/a_b_c*d?q=1",
            "#hashtag and a [ ] box",
            "| a | b |\n|---|---|\n| 1 | 2 |",
            "x -- y",
        ] {
            assert_eq!(plain(text), text);
        }
    }

    #[test]
    fn line_ends_stay() {
        assert_eq!(plain("**a**\r\nb\r\n"), "a\r\nb\r\n");
        assert_eq!(plain("# a\n"), "a\n");
    }
}
