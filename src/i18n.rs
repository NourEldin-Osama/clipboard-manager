//! Interface language and how dates and times are shown.
//!
//! Every text of the window comes from `locales/<language>/main.ftl`
//! (Project Fluent), by a key that says what the text is for. `t!` takes the
//! key, and for a text with values in it, the values by name:
//! `t!("time-minutes-ago", count = 5)`. A text missing in a language shows in
//! English. `docs/TRANSLATING.md` explains how to add a language.
//!
//! An error uses `log_error!` instead: it returns the text to show, and
//! writes the same text in English to the log, so the log reads the same
//! for everyone.
//!
//! `tr` is the older way, by English text; it goes once every
//! text has moved to the .ftl files.
//!
//! Items store their time as UTC Unix seconds; `local` converts each one with
//! the offset that applied at that moment (daylight saving included).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use chrono::{DateTime, Datelike, Local, TimeZone, Timelike};
use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};

use crate::config::{Language, TimeFormat};

mod ar;

static ARABIC: AtomicBool = AtomicBool::new(false);
static H12: AtomicBool = AtomicBool::new(false);

/// The text files, built into the exe.
const ENGLISH_TEXTS: &str = include_str!("../locales/en/main.ftl");
const ARABIC_TEXTS: &str = include_str!("../locales/ar/main.ftl");

pub fn set(language: Language, time: TimeFormat) {
    ARABIC.store(language == Language::Arabic, Ordering::Relaxed);
    H12.store(time == TimeFormat::H12, Ordering::Relaxed);
}

/// Right-to-left layout (Arabic).
pub fn rtl() -> bool {
    ARABIC.load(Ordering::Relaxed)
}

/// The text for `key` in the interface language: `t!("toast-copied")`, or
/// with values by name: `t!("status-items", count = n)`.
macro_rules! t {
    ($key:literal) => {
        $crate::i18n::text($key, None)
    };
    ($key:literal, $($name:ident = $value:expr),+ $(,)?) => {{
        let mut args = ::fluent_bundle::FluentArgs::new();
        $(args.set(stringify!($name), $value);)+
        $crate::i18n::text($key, Some(&args))
    }};
}
pub(crate) use t;

/// Like `t!`, for an error: writes the text to the log in English, and
/// returns it in the interface language, to show.
macro_rules! log_error {
    ($key:literal) => {{
        ::log::error!(
            "{}",
            $crate::i18n::text_in($crate::config::Language::English, $key, None)
        );
        $crate::i18n::text($key, None)
    }};
    ($key:literal, $($name:ident = $value:expr),+ $(,)?) => {{
        let mut args = ::fluent_bundle::FluentArgs::new();
        $(args.set(stringify!($name), $value);)+
        ::log::error!(
            "{}",
            $crate::i18n::text_in($crate::config::Language::English, $key, Some(&args))
        );
        $crate::i18n::text($key, Some(&args))
    }};
}
pub(crate) use log_error;

type Bundle = FluentBundle<FluentResource>;

fn bundle(language: Language) -> &'static Bundle {
    static BUNDLES: OnceLock<[Bundle; 2]> = OnceLock::new();
    let bundles = BUNDLES.get_or_init(|| [load("en", ENGLISH_TEXTS), load("ar", ARABIC_TEXTS)]);
    match language {
        Language::English => &bundles[0],
        Language::Arabic => &bundles[1],
    }
}

/// Parses one language's texts. The files are part of the build and checked
/// by the tests below, so a broken file is a bug in this build.
fn load(locale: &str, source: &str) -> Bundle {
    let resource = FluentResource::try_new(source.to_string())
        .unwrap_or_else(|(_, errors)| panic!("locales/{locale}/main.ftl: {errors:?}"));
    let mut bundle = Bundle::new_concurrent(vec![locale.parse().expect("a language tag")]);
    // Fluent marks each value's text direction with invisible characters that
    // egui would draw as boxes; mixed Arabic and English is laid out by
    // `app::bidi` instead.
    bundle.set_use_isolating(false);
    if let Err(errors) = bundle.add_resource(resource) {
        panic!("locales/{locale}/main.ftl: {errors:?}");
    }
    bundle
}

/// The text for `key` in the interface language; `t!` calls this.
pub fn text(key: &str, args: Option<&FluentArgs>) -> String {
    let language = if rtl() {
        Language::Arabic
    } else {
        Language::English
    };
    text_in(language, key, args)
}

/// The text for `key` in `language`, or in English when that language lacks
/// it. A key no language has shows as itself, so a mistake stays visible.
pub fn text_in(language: Language, key: &str, args: Option<&FluentArgs>) -> String {
    for bundle in [bundle(language), bundle(Language::English)] {
        let Some(pattern) = bundle.get_message(key).and_then(|m| m.value()) else {
            continue;
        };
        let mut errors = Vec::new();
        let text = bundle.format_pattern(pattern, args, &mut errors);
        if !errors.is_empty() {
            log::warn!("text {key}: {errors:?}");
        }
        return text.into_owned();
    }
    log::warn!("no text for {key}");
    key.to_string()
}

fn arabic() -> &'static HashMap<&'static str, &'static str> {
    static TABLE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    TABLE.get_or_init(|| ar::TABLE.iter().copied().collect())
}

/// `en` in the interface language.
pub fn tr(en: &str) -> &str {
    if rtl() {
        if let Some(t) = arabic().get(en) {
            return t;
        }
    }
    en
}

/// A stored UTC timestamp in the user's time zone.
pub fn local(ts: i64) -> DateTime<Local> {
    Local
        .timestamp_opt(ts, 0)
        .single()
        .unwrap_or_else(|| Local.timestamp_opt(0, 0).unwrap())
}

/// "14:05", or "2:05 PM" with the 12-hour clock.
pub fn clock(dt: &DateTime<Local>) -> String {
    if H12.load(Ordering::Relaxed) {
        let (pm, h) = dt.hour12();
        let time = format!("{h}:{:02}", dt.minute());
        if pm {
            t!("time-clock-pm", time = time)
        } else {
            t!("time-clock-am", time = time)
        }
    } else {
        format!("{:02}:{:02}", dt.hour(), dt.minute())
    }
}

pub fn weekday(dt: &DateTime<Local>) -> String {
    match dt.weekday() {
        chrono::Weekday::Mon => t!("weekday-mon"),
        chrono::Weekday::Tue => t!("weekday-tue"),
        chrono::Weekday::Wed => t!("weekday-wed"),
        chrono::Weekday::Thu => t!("weekday-thu"),
        chrono::Weekday::Fri => t!("weekday-fri"),
        chrono::Weekday::Sat => t!("weekday-sat"),
        chrono::Weekday::Sun => t!("weekday-sun"),
    }
}

pub fn month(dt: &DateTime<Local>) -> String {
    match dt.month() {
        1 => t!("month-jan"),
        2 => t!("month-feb"),
        3 => t!("month-mar"),
        4 => t!("month-apr"),
        5 => t!("month-may"),
        6 => t!("month-jun"),
        7 => t!("month-jul"),
        8 => t!("month-aug"),
        9 => t!("month-sep"),
        10 => t!("month-oct"),
        11 => t!("month-nov"),
        _ => t!("month-dec"),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use fluent_syntax::ast;

    use super::*;

    /// Every message of one .ftl file, with the names of the values it uses.
    fn messages(source: &str) -> BTreeMap<String, BTreeSet<String>> {
        let resource = fluent_syntax::parser::parse(source)
            .unwrap_or_else(|(_, errors)| panic!("the file does not parse: {errors:?}"));
        let mut found = BTreeMap::new();
        for entry in &resource.body {
            if let ast::Entry::Message(message) = entry {
                let mut names = BTreeSet::new();
                if let Some(pattern) = &message.value {
                    pattern_names(pattern, &mut names);
                }
                found.insert(message.id.name.to_string(), names);
            }
        }
        found
    }

    fn pattern_names(pattern: &ast::Pattern<&str>, names: &mut BTreeSet<String>) {
        for element in &pattern.elements {
            if let ast::PatternElement::Placeable { expression } = element {
                expression_names(expression, names);
            }
        }
    }

    fn expression_names(expression: &ast::Expression<&str>, names: &mut BTreeSet<String>) {
        match expression {
            ast::Expression::Inline(inline) => inline_names(inline, names),
            ast::Expression::Select { selector, variants } => {
                inline_names(selector, names);
                for variant in variants {
                    pattern_names(&variant.value, names);
                }
            }
        }
    }

    fn inline_names(inline: &ast::InlineExpression<&str>, names: &mut BTreeSet<String>) {
        match inline {
            ast::InlineExpression::VariableReference { id } => {
                names.insert(id.name.to_string());
            }
            ast::InlineExpression::Placeable { expression } => expression_names(expression, names),
            ast::InlineExpression::FunctionReference { arguments, .. } => {
                for argument in &arguments.positional {
                    inline_names(argument, names);
                }
                for argument in &arguments.named {
                    inline_names(&argument.value, names);
                }
            }
            _ => {}
        }
    }

    /// Every key passed to `t!` or `log_error!` in the source code,
    /// skipping comments.
    fn keys_in_code() -> BTreeSet<String> {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut keys = BTreeSet::new();
        for entry in walkdir::WalkDir::new(src) {
            let entry = entry.unwrap();
            if entry.path().extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let code: String = std::fs::read_to_string(entry.path())
                .unwrap()
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            let calls = ["t!(", "log_error!("]
                .into_iter()
                .flat_map(|name| code.match_indices(name));
            for (at, name) in calls {
                // Not the end of another macro, like format!( or assert!(.
                let before = code[..at].chars().next_back();
                if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                    continue;
                }
                let rest = code[at + name.len()..].trim_start();
                let Some(quoted) = rest.strip_prefix('"') else {
                    continue;
                };
                let key = &quoted[..quoted.find('"').unwrap_or(0)];
                // Keys are lower-case words joined by dashes; anything else
                // (like this test's own search pattern) is not a key.
                let is_key = !key.is_empty()
                    && key
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
                if is_key {
                    keys.insert(key.to_string());
                }
            }
        }
        keys
    }

    #[test]
    fn every_language_has_every_text_with_the_same_values() {
        let english = messages(ENGLISH_TEXTS);
        let arabic = messages(ARABIC_TEXTS);
        let missing: Vec<_> = english
            .keys()
            .filter(|k| !arabic.contains_key(*k))
            .collect();
        let extra: Vec<_> = arabic
            .keys()
            .filter(|k| !english.contains_key(*k))
            .collect();
        assert!(missing.is_empty(), "not in Arabic: {missing:?}");
        assert!(extra.is_empty(), "only in Arabic: {extra:?}");
        for (key, names) in &english {
            assert_eq!(names, &arabic[key], "values of {key}");
        }
    }

    #[test]
    fn the_code_and_the_texts_match() {
        let defined: BTreeSet<String> = messages(ENGLISH_TEXTS).into_keys().collect();
        let used = keys_in_code();
        let undefined: Vec<_> = used.difference(&defined).collect();
        let unused: Vec<_> = defined.difference(&used).collect();
        assert!(
            undefined.is_empty(),
            "used but not in the .ftl files: {undefined:?}"
        );
        assert!(
            unused.is_empty(),
            "in the .ftl files but never used: {unused:?}"
        );
    }

    #[test]
    fn plurals_follow_each_language() {
        let ago = |language, count: i64| {
            let mut args = FluentArgs::new();
            args.set("count", count);
            text_in(language, "time-minutes-ago", Some(&args))
        };
        assert_eq!(ago(Language::English, 1), "1 minute ago");
        assert_eq!(ago(Language::English, 5), "5 minutes ago");
        assert_eq!(ago(Language::Arabic, 1), "منذ دقيقة");
        assert_eq!(ago(Language::Arabic, 2), "منذ دقيقتين");
        assert_eq!(ago(Language::Arabic, 5), "منذ 5 دقائق");
        assert_eq!(ago(Language::Arabic, 11), "منذ 11 دقيقة");
    }

    #[test]
    fn a_missing_text_falls_back_to_english_then_to_its_key() {
        assert_eq!(
            text_in(Language::Arabic, "no-such-key", None),
            "no-such-key"
        );
    }

    /// Every translation keeps the placeholders of its English text.
    #[test]
    fn placeholders_match() {
        let names = |s: &str| {
            let mut v: Vec<String> = s
                .split('{')
                .skip(1)
                .filter_map(|p| p.split_once('}').map(|(n, _)| n.to_string()))
                .collect();
            v.sort();
            v
        };
        for (en, ar) in ar::TABLE {
            assert_eq!(names(en), names(ar), "{en}");
        }
    }

    #[test]
    fn no_duplicate_keys() {
        let mut seen = std::collections::HashSet::new();
        for (en, _) in ar::TABLE {
            assert!(seen.insert(*en), "duplicate: {en}");
        }
    }

    #[test]
    fn twelve_hour_clock() {
        set(Language::English, TimeFormat::H12);
        let dt = Local.with_ymd_and_hms(2026, 10, 21, 14, 5, 0).unwrap();
        assert_eq!(clock(&dt), "2:05 PM");
        set(Language::English, TimeFormat::H24);
        assert_eq!(clock(&dt), "14:05");
    }
}
