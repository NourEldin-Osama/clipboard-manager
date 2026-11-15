//! Colors the code in copies (D95): guesses the language from the text
//! itself, colors it with bat's languages (two-face, run by syntect with
//! Oniguruma) on a thread of its own, and gives each piece of the code its
//! role. The window paints each role in the theme's color for it.

use std::collections::HashMap;
use std::ops::Range;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::str::FromStr;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

use syntect::easy::HighlightLines;
use syntect::highlighting::{
    Color, ScopeSelectors, StyleModifier, Theme, ThemeItem, ThemeSettings,
};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use crate::store::Item;

/// Longer code stays plain: coloring it would take long and a lot of
/// memory, and nobody reads that much in the window.
const MOST_LINES: usize = 2000;
const MOST_BYTES: usize = 100_000;

/// A line this long is minified code, which colors slowly and reads no
/// better for it.
const LONGEST_LINE: usize = 5000;

/// How many copies keep their colors at once; the window asks again for
/// one it shows after they were let go.
const MOST_KEPT: usize = 500;

/// What a piece of code is, for its color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// Names, operators and punctuation: the text's own color.
    Text,
    Keyword,
    /// Functions, and the keys of JSON and YAML.
    Function,
    String,
    /// Numbers and constants such as true and null.
    Number,
    Type,
    Comment,
}

impl Role {
    const ALL: [Role; 7] = [
        Role::Text,
        Role::Keyword,
        Role::Function,
        Role::String,
        Role::Number,
        Role::Type,
        Role::Comment,
    ];
}

/// Which scopes of bat's languages take which role. The longer scope wins,
/// so operators stay plain although they are keywords too.
const RULES: [(&str, Role); 8] = [
    ("comment, punctuation.definition.comment", Role::Comment),
    (
        "string, punctuation.definition.string, constant.character.escape",
        Role::String,
    ),
    (
        "constant.numeric, constant.language, constant.character, constant.other, \
         support.constant",
        Role::Number,
    ),
    (
        "keyword, storage, variable.language, entity.name.tag",
        Role::Keyword,
    ),
    ("keyword.operator, punctuation", Role::Text),
    (
        "entity.name.function, support.function, variable.function, \
         meta.function-call.identifier",
        Role::Function,
    ),
    (
        "support.type.property-name, meta.mapping.key string",
        Role::Function,
    ),
    (
        "entity.name.type, entity.name.class, entity.name.struct, entity.name.enum, \
         entity.name.interface, support.type, support.class, entity.other.inherited-class, \
         entity.other.attribute-name",
        Role::Type,
    ),
];

/// A copy's code with the role of each piece that isn't plain.
#[derive(Debug, PartialEq)]
pub struct Colored {
    /// The code as colored: its lines joined by "\n".
    pub text: String,
    /// Pieces of `text` by byte range, in order, each with its role.
    pub pieces: Vec<(Range<usize>, Role)>,
}

/// The language of `code`, guessed from the text alone (a copy has no file
/// name), as the extension bat knows it by; none when unsure, and the code
/// then stays plain.
pub fn language(code: &str) -> Option<&'static str> {
    let text = code.trim();
    let first = text.lines().next().unwrap_or_default();
    if let Some(program) = first.strip_prefix("#!") {
        let found = [
            ("python", "py"),
            ("node", "js"),
            ("pwsh", "ps1"),
            ("ruby", "rb"),
            ("php", "php"),
        ]
        .iter()
        .find(|(name, _)| program.contains(name));
        return Some(found.map_or("sh", |(_, language)| language));
    }
    if first.starts_with("<?php") {
        return Some("php");
    }
    if first.starts_with("<?xml") {
        return Some("xml");
    }
    if first.starts_with("diff --git") || (first.starts_with("--- ") && text.contains("\n+++ ")) {
        return Some("diff");
    }
    if first.starts_with("FROM ") && text.lines().any(|l| l.starts_with("RUN ")) {
        return Some("Dockerfile");
    }
    if (text.starts_with('{') || text.starts_with('['))
        && serde_json::from_str::<serde_json::Value>(text).is_ok()
    {
        return Some("json");
    }
    let start = first.to_ascii_lowercase();
    if start.starts_with("<!doctype html") || (text.starts_with('<') && text.contains("</")) {
        return Some("html");
    }
    let has = |marks: &[&str]| marks.iter().filter(|m| text.contains(*m)).count();
    let lines = |test: &dyn Fn(&str) -> bool| text.lines().filter(|l| test(l.trim())).count();
    // A Python block starts on a line that ends with a colon.
    let python_block = |line: &str| {
        let starts = [
            "def ", "class ", "if ", "for ", "while ", "with ", "elif ", "else", "try", "except",
        ];
        line.ends_with(':') && starts.iter().any(|s| line.starts_with(s))
    };
    // Text with braces or semicolons is no YAML or TOML, which have many
    // lines that look like theirs.
    let plain = !text.contains(['{', ';']);
    let ends = |line: &str| line == "end";
    let scores = [
        (
            "sql",
            2 * has(&[
                "SELECT ",
                " FROM ",
                "WHERE ",
                " JOIN ",
                "GROUP BY",
                "ORDER BY",
                "INSERT INTO",
                "CREATE TABLE",
                "UPDATE ",
                " SET ",
                "VALUES",
            ]),
        ),
        (
            "ps1",
            2 * has(&[
                "Get-",
                "Set-",
                "Write-Host",
                "Where-Object",
                "ForEach-Object",
                "Select-Object",
                "-ErrorAction",
                "$_",
                "New-Object",
            ]),
        ),
        (
            "ts",
            2 * has(&[
                "interface ",
                ": string",
                ": number",
                ": boolean",
                "export type ",
                "readonly ",
                ": void",
            ]),
        ),
        (
            "cs",
            2 * has(&[
                "using System",
                "namespace ",
                "Console.Write",
                "static void Main",
                "async Task",
                "{ get; set; }",
            ]),
        ),
        (
            "java",
            2 * has(&[
                "System.out.print",
                "import java.",
                "public static void main(String",
                "@Override",
            ]),
        ),
        ("go", 2 * has(&["package main", "func ", ":= ", "fmt."])),
        (
            "cpp",
            2 * has(&[
                "#include",
                "std::",
                "int main(",
                "cout <<",
                "nullptr",
                "template<",
                "template <",
            ]),
        ),
        (
            "rb",
            has(&["puts ", "require '", "attr_accessor", " do |", ".each do"]) + 2 * lines(&ends),
        ),
        (
            "php",
            2 * has(&["$this->", "<?php", "echo $", "function __construct"]),
        ),
        (
            "rs",
            has(&[
                "fn ",
                "let mut ",
                "impl ",
                "pub fn ",
                "-> ",
                "&mut ",
                "println!",
                "match ",
                "use std::",
                "::new(",
                "#[derive",
            ]),
        ),
        (
            "py",
            has(&[
                "def ", "import ", "from ", "elif ", "self.", "print(", "lambda ", "None",
                "\"\"\"", "__init__",
            ]) + lines(&python_block),
        ),
        (
            "js",
            has(&[
                "const ",
                "let ",
                "function ",
                "=> ",
                "console.log",
                "await ",
                "export ",
                "require(",
                "document.",
                "===",
            ]),
        ),
        (
            "sh",
            has(&[
                "echo ", "fi\n", "; then", "$(", "esac", "sudo ", "apt-get ", "chmod ", "grep ",
                "export ",
            ]),
        ),
        (
            "css",
            lines(&|l: &str| l.ends_with(';') && l.contains(": ") && !l.contains('('))
                + 2 * has(&["@media", "px;", "rem;"]),
        ),
        (
            "yaml",
            if plain {
                lines(&|l: &str| {
                    let pair = l
                        .split_once(": ")
                        .or_else(|| l.strip_suffix(':').map(|key| (key, "")))
                        .is_some_and(|(key, _)| !key.is_empty() && !key.contains(' '));
                    pair || l.starts_with("- ")
                })
            } else {
                0
            },
        ),
        (
            "toml",
            if plain
                && text
                    .lines()
                    .any(|l| l.starts_with('[') && l.trim_end().ends_with(']'))
            {
                lines(&|l: &str| {
                    (l.starts_with('[') && l.ends_with(']'))
                        || l.split_once(" = ")
                            .is_some_and(|(key, _)| !key.contains(' '))
                })
            } else {
                0
            },
        ),
    ];
    let mut best: Option<(&'static str, usize)> = None;
    for (language, score) in scores {
        if score >= 2 && best.is_none_or(|(_, top)| score > top) {
            best = Some((language, score));
        }
    }
    best.map(|(language, _)| language)
}

/// A syntect theme whose colors are the roles' numbers, so a style's color
/// says its role.
fn role_theme() -> Theme {
    let role_color = |role: Role| Color {
        r: Role::ALL.iter().position(|r| *r == role).unwrap_or(0) as u8,
        g: 0,
        b: 0,
        a: 0xFF,
    };
    Theme {
        name: Some("Roles".into()),
        author: None,
        settings: ThemeSettings {
            foreground: Some(role_color(Role::Text)),
            ..Default::default()
        },
        scopes: RULES
            .iter()
            .map(|(scopes, role)| ThemeItem {
                scope: ScopeSelectors::from_str(scopes).expect("the rules' scopes"),
                style: StyleModifier {
                    foreground: Some(role_color(*role)),
                    background: None,
                    font_style: None,
                },
            })
            .collect(),
    }
}

/// `text` colored, or none when it is too long, its language unknown, or
/// its language's patterns fail.
fn color(set: &SyntaxSet, theme: &Theme, text: &str) -> Option<Colored> {
    if text.len() > MOST_BYTES
        || text.lines().count() > MOST_LINES
        || text.lines().any(|l| l.len() > LONGEST_LINE)
    {
        return None;
    }
    let syntax = set.find_syntax_by_extension(language(text)?)?;
    let text = text.lines().collect::<Vec<_>>().join("\n");
    let mut colorer = HighlightLines::new(syntax, theme);
    let mut pieces: Vec<(Range<usize>, Role)> = Vec::new();
    let mut at = 0;
    for line in LinesWithEndings::from(&text) {
        for (style, piece) in colorer.highlight_line(line, set).ok()? {
            let role = Role::ALL
                .get(usize::from(style.foreground.r))
                .copied()
                .unwrap_or(Role::Text);
            let end = at + piece.len();
            if role != Role::Text && !piece.trim().is_empty() {
                match pieces.last_mut() {
                    Some((range, last)) if *last == role && range.end == at => range.end = end,
                    _ => pieces.push((at..end, role)),
                }
            }
            at = end;
        }
    }
    Some(Colored { text, pieces })
}

/// Colors code on a thread of its own: loading the languages and coloring
/// a long text take long enough to stall the window.
struct Colorer {
    requests: Sender<(u64, Arc<str>)>,
    answers: Receiver<(u64, Option<Arc<Colored>>)>,
}

impl Colorer {
    fn spawn(wake: impl Fn() + Send + 'static) -> Self {
        let (requests, inbox) = channel::<(u64, Arc<str>)>();
        let (outbox, answers) = channel();
        std::thread::Builder::new()
            .name("code colors".into())
            .spawn(move || {
                // Loaded with the first code to color, not at start.
                let mut loaded: Option<(SyntaxSet, Theme)> = None;
                while let Ok((id, text)) = inbox.recv() {
                    let (set, theme) = loaded
                        .get_or_insert_with(|| (two_face::syntax::extra_newlines(), role_theme()));
                    let colored = catch_unwind(AssertUnwindSafe(|| color(set, theme, &text)))
                        .unwrap_or_else(|_| {
                            log::error!("coloring a copy's code crashed; it stays plain");
                            None
                        });
                    if outbox.send((id, colored.map(Arc::new))).is_err() {
                        return;
                    }
                    wake();
                }
            })
            .expect("spawn the code colors thread");
        Self { requests, answers }
    }
}

/// The colors of the code in copies, kept by copy, from the colorer.
pub struct CodeColors {
    colorer: Colorer,
    /// By copy: its colors, or none while they are coming or when its
    /// code stays plain.
    found: HashMap<u64, Option<Arc<Colored>>>,
}

impl CodeColors {
    pub fn spawn(wake: impl Fn() + Send + 'static) -> Self {
        Self {
            colorer: Colorer::spawn(wake),
            found: HashMap::new(),
        }
    }

    /// The colors of `it`'s code once colored; the first time, asks for them
    /// (the window shows the code plain meanwhile).
    pub fn of(&mut self, it: &Item) -> Option<Arc<Colored>> {
        while let Ok((id, colored)) = self.colorer.answers.try_recv() {
            self.found.insert(id, colored);
        }
        if let Some(found) = self.found.get(&it.id) {
            return found.clone();
        }
        if self.found.len() >= MOST_KEPT {
            self.found.clear();
        }
        self.found.insert(it.id, None);
        let _ = self.colorer.requests.send((it.id, it.text.clone()));
        None
    }

    /// Lets go of the colors of the copies `ids`, which were removed.
    pub fn forget(&mut self, ids: &[u64]) {
        for id in ids {
            self.found.remove(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn languages() -> (SyntaxSet, Theme) {
        (two_face::syntax::extra_newlines(), role_theme())
    }

    /// Code in each language, as it might be copied, with what it is.
    const SAMPLES: [(&str, &str); 20] = [
        ("py", "import json\n\ndef load(path):\n    with open(path) as f:\n        return json.load(f)\n"),
        ("js", "const ids = [1, 2, 3];\nconsole.log(ids.map((n) => n * 2));\n"),
        ("ts", "interface User {\n  name: string;\n  age: number;\n}\n"),
        ("rs", "fn main() {\n    let mut total = 0;\n    println!(\"{total}\");\n}\n"),
        ("sql", "SELECT name, total FROM orders WHERE total > 1000 ORDER BY total DESC;"),
        ("json", "{\n  \"name\": \"clipboard\",\n  \"private\": true\n}"),
        ("cpp", "#include <iostream>\nint main() {\n    std::cout << \"hi\";\n}\n"),
        ("sh", "#!/bin/bash\necho \"start\"\nif [ -f a.txt ]; then\n  cat a.txt\nfi\n"),
        ("yaml", "name: build\non: push\njobs:\n  test:\n    runs-on: windows-latest\n"),
        ("cs", "using System;\nnamespace Demo {\n    class Program {\n        static void Main() { Console.WriteLine(1); }\n    }\n}\n"),
        ("ps1", "Get-ChildItem -Path $folder | Where-Object { $_.Length -gt 10MB }\n"),
        ("html", "<!DOCTYPE html>\n<html>\n<body><p>Hi</p></body>\n</html>\n"),
        ("css", ".card {\n  color: red;\n  padding: 4px;\n}\n"),
        ("go", "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tx := 1\n\tfmt.Println(x)\n}\n"),
        ("java", "public class App {\n    public static void main(String[] args) {\n        System.out.println(1);\n    }\n}\n"),
        ("php", "<?php\nclass A {\n    function __construct() { $this->x = 1; }\n}\n"),
        ("rb", "class Greeter\n  def hi\n    puts 'hi'\n  end\nend\n"),
        ("toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n"),
        ("xml", "<?xml version=\"1.0\"?>\n<config><item id=\"1\"/></config>\n"),
        ("Dockerfile", "FROM rust:1.95\nRUN cargo build --release\nCOPY . /app\n"),
    ];

    #[test]
    fn the_language_is_guessed_from_the_code() {
        for (expected, code) in SAMPLES {
            assert_eq!(language(code), Some(expected), "{code}");
        }
        assert_eq!(language("Meeting moved to Thursday at 10:30."), None);
        assert_eq!(language("x = 1"), None, "too little to tell");
    }

    /// Every language the guess gives is one of bat's.
    #[test]
    fn every_guessed_language_is_one_bat_knows() {
        let (set, _) = languages();
        for (language, _) in SAMPLES {
            assert!(
                set.find_syntax_by_extension(language).is_some(),
                "{language} is not one of bat's"
            );
        }
    }

    fn roles(colored: &Colored, role: Role) -> Vec<&str> {
        colored
            .pieces
            .iter()
            .filter(|(_, r)| *r == role)
            .map(|(range, _)| colored.text[range.clone()].trim())
            .collect()
    }

    #[test]
    fn each_piece_of_code_gets_its_role() {
        let (set, theme) = languages();
        let code = "def load(path):\r\n    # the file\r\n    return open(path, \"r\", 2)\r\n";
        let colored = color(&set, &theme, code).expect("Python colors");
        assert_eq!(
            colored.text,
            "def load(path):\n    # the file\n    return open(path, \"r\", 2)"
        );
        assert!(roles(&colored, Role::Keyword).contains(&"def"));
        assert!(roles(&colored, Role::Keyword).contains(&"return"));
        assert!(roles(&colored, Role::Function).contains(&"load"));
        assert_eq!(roles(&colored, Role::Comment), ["# the file"]);
        assert_eq!(roles(&colored, Role::String), ["\"r\""]);
        assert_eq!(roles(&colored, Role::Number), ["2"]);
        // Pieces never overlap and stay inside the text.
        for pair in colored.pieces.windows(2) {
            assert!(pair[0].0.end <= pair[1].0.start);
        }
        assert!(colored
            .pieces
            .last()
            .is_some_and(|(r, _)| r.end <= colored.text.len()));
        // JSON's keys take the functions' color, its values their own.
        let json = color(&set, &theme, "{\n  \"name\": \"x\",\n  \"count\": 3\n}").expect("JSON");
        assert!(roles(&json, Role::Function)
            .iter()
            .any(|k| k.contains("name")));
        assert_eq!(roles(&json, Role::Number), ["3"]);
    }

    #[test]
    fn long_or_unknown_code_stays_plain() {
        let (set, theme) = languages();
        let long = "let x = 1;\n".repeat(MOST_LINES + 1);
        assert_eq!(color(&set, &theme, &long), None, "too many lines");
        let minified = format!("const a = [{}];", "1,".repeat(LONGEST_LINE));
        assert_eq!(color(&set, &theme, &minified), None, "a minified line");
        assert_eq!(color(&set, &theme, "just (some) text; here"), None);
    }
}
