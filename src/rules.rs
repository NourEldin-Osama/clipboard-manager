//! Rules for copies that aren't recorded: everything from a program, or text
//! that matches a pattern, from one program or from any.
//!
//! A program is its file name, as in `chrome.exe`. A simple pattern uses `*`
//! for any text and `?` for one character, ignores case, and must match the
//! whole copy, so `*word*` finds a word anywhere. A regular expression is
//! found anywhere in the copy, as regular expressions usually are.

use regex::Regex;
use serde::{Deserialize, Serialize};

/// One rule, as set in Settings.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Rule {
    /// The program's file name, as in `chrome.exe`; empty for every program.
    pub app: String,
    /// The text to skip; empty skips everything from `app`.
    pub pattern: String,
    /// `pattern` is a regular expression instead of a simple pattern.
    pub is_regex: bool,
}

/// Why a rule can't be used.
#[derive(Debug, PartialEq)]
pub enum Problem {
    /// Neither a program nor a pattern: it would skip every copy.
    Empty,
    /// The regular expression doesn't compile; the text says why.
    BadPattern(String),
}

impl Rule {
    /// Whether this rule skips every copy from `app`.
    pub fn skips_app(&self, app: &str) -> bool {
        self.pattern.trim().is_empty() && self.app.trim().eq_ignore_ascii_case(app)
    }

    /// The compiled pattern (`None` when the rule has none), or what is wrong
    /// with the rule.
    pub fn compile(&self) -> Result<Option<Regex>, Problem> {
        let pattern = self.pattern.trim();
        if pattern.is_empty() {
            return if self.app.trim().is_empty() {
                Err(Problem::Empty)
            } else {
                Ok(None)
            };
        }
        let source = if self.is_regex {
            pattern.to_string()
        } else {
            simple_pattern(pattern)
        };
        Regex::new(&source)
            .map(Some)
            .map_err(|e| Problem::BadPattern(e.to_string()))
    }
}

/// A simple pattern as a regular expression: `*` any text, `?` one
/// character, the whole text, any case.
fn simple_pattern(pattern: &str) -> String {
    let mut out = String::from("(?is)^");
    for c in pattern.chars() {
        match c {
            '*' => out.push_str(".*"),
            '?' => out.push('.'),
            c => out.push_str(&regex::escape(&c.to_string())),
        }
    }
    out.push('$');
    out
}

/// The rules ready to match copies. Rules that can't be used are left out.
#[derive(Default)]
pub struct Rules(Vec<(String, Option<Regex>)>);

impl Rules {
    pub fn new(rules: &[Rule]) -> Self {
        Self(
            rules
                .iter()
                .filter_map(|r| Some((r.app.trim().to_lowercase(), r.compile().ok()?)))
                .collect(),
        )
    }

    /// Whether a copy is skipped. `app` is the program it came from, if
    /// known, and `text` its text (`None` for images and files, which only
    /// rules without a pattern skip).
    pub fn skips(&self, app: Option<&str>, text: Option<&str>) -> bool {
        let app = app.map(str::to_lowercase);
        self.0.iter().any(|(rule_app, pattern)| {
            let app_matches = rule_app.is_empty() || app.as_deref() == Some(rule_app.as_str());
            app_matches
                && match (pattern, text) {
                    (None, _) => true,
                    (Some(pattern), Some(text)) => {
                        // Simple patterns compare the whole copy, without the
                        // spaces and line breaks around it.
                        pattern.is_match(text) || pattern.is_match(text.trim())
                    }
                    (Some(_), None) => false,
                }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(app: &str, pattern: &str, is_regex: bool) -> Rule {
        Rule {
            app: app.into(),
            pattern: pattern.into(),
            is_regex,
        }
    }

    #[test]
    fn a_program_rule_skips_everything_from_that_program() {
        let rules = Rules::new(&[rule("KeePass.exe", "", false)]);
        assert!(rules.skips(Some("keepass.exe"), Some("anything")));
        assert!(
            rules.skips(Some("KEEPASS.EXE"), None),
            "images and files too"
        );
        assert!(!rules.skips(Some("notepad.exe"), Some("anything")));
        assert!(!rules.skips(None, Some("anything")), "an unknown program");
    }

    #[test]
    fn simple_patterns_match_the_whole_copy() {
        let rules = Rules::new(&[rule("", "*@company.com", false)]);
        assert!(rules.skips(None, Some("Ann@Company.com")), "any case");
        assert!(rules.skips(Some("outlook.exe"), Some("  bob@company.com\n")));
        assert!(!rules.skips(None, Some("bob@company.com is here")));
        let rules = Rules::new(&[rule("", "*secret*", false)]);
        assert!(rules.skips(None, Some("my secret plan")));
        let rules = Rules::new(&[rule("", "pin ????", false)]);
        assert!(rules.skips(None, Some("PIN 1234")));
        assert!(!rules.skips(None, Some("PIN 12345")));
        let rules = Rules::new(&[rule("", "a.b", false)]);
        assert!(!rules.skips(None, Some("axb")), "a dot is just a dot");
        assert!(!rules.skips(None, None), "patterns never skip images");
    }

    #[test]
    fn regular_expressions_match_anywhere() {
        let rules = Rules::new(&[rule("", r"\d{4}-\d{4}", true)]);
        assert!(rules.skips(None, Some("card 1234-5678 expires")));
        assert!(!rules.skips(None, Some("card 12-34")));
        let anchored = Rules::new(&[rule("", "^ticket", true)]);
        assert!(!anchored.skips(None, Some("my ticket")));
    }

    #[test]
    fn a_pattern_for_one_program_skips_only_there() {
        let rules = Rules::new(&[rule("chrome.exe", "*password*", false)]);
        assert!(rules.skips(Some("chrome.exe"), Some("my password")));
        assert!(!rules.skips(Some("notepad.exe"), Some("my password")));
        assert!(!rules.skips(Some("chrome.exe"), Some("hello")));
    }

    #[test]
    fn rules_that_cant_be_used_are_left_out() {
        assert_eq!(rule("", "", false).compile().err(), Some(Problem::Empty));
        assert!(matches!(
            rule("", "(unclosed", true).compile(),
            Err(Problem::BadPattern(_))
        ));
        let rules = Rules::new(&[rule("", "", false), rule("", "(unclosed", true)]);
        assert!(!rules.skips(Some("chrome.exe"), Some("anything")));
    }

    #[test]
    fn skips_app_means_a_rule_without_a_pattern() {
        assert!(rule("Chrome.exe", "", false).skips_app("chrome.exe"));
        assert!(!rule("chrome.exe", "*x*", false).skips_app("chrome.exe"));
        assert!(!rule("", "", false).skips_app("chrome.exe"));
    }
}
