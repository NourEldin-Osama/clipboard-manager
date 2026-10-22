//! Interface language and how dates and times are shown.
//!
//! Strings are looked up by their English text, so call sites stay readable:
//! `tr("Copy")`, or `trf("{n} items", &[("n", &n)])` for text with values in it.
//! English is the fallback for anything not translated yet.
//!
//! Items store their time as UTC Unix seconds; `local` converts each one with
//! the offset that applied at that moment (daylight saving included).

use std::collections::HashMap;
use std::fmt::Display;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use chrono::{DateTime, Datelike, Local, TimeZone, Timelike};

use crate::config::{Language, TimeFormat};

mod ar;

static ARABIC: AtomicBool = AtomicBool::new(false);
static H12: AtomicBool = AtomicBool::new(false);

pub fn set(language: Language, time: TimeFormat) {
    ARABIC.store(language == Language::Arabic, Ordering::Relaxed);
    H12.store(time == TimeFormat::H12, Ordering::Relaxed);
}

/// Right-to-left layout (Arabic).
pub fn rtl() -> bool {
    ARABIC.load(Ordering::Relaxed)
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

/// `en` translated, with each `{name}` replaced by its value.
pub fn trf(en: &str, args: &[(&str, &dyn Display)]) -> String {
    let mut s = tr(en).to_string();
    for (name, value) in args {
        s = s.replace(&format!("{{{name}}}"), &value.to_string());
    }
    s
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
        format!(
            "{h}:{:02} {}",
            dt.minute(),
            tr(if pm { "PM" } else { "AM" })
        )
    } else {
        format!("{:02}:{:02}", dt.hour(), dt.minute())
    }
}

pub fn weekday(dt: &DateTime<Local>) -> &'static str {
    const EN: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    const AR: [&str; 7] = [
        "الإثنين",
        "الثلاثاء",
        "الأربعاء",
        "الخميس",
        "الجمعة",
        "السبت",
        "الأحد",
    ];
    let i = dt.weekday().num_days_from_monday() as usize;
    if rtl() {
        AR[i]
    } else {
        EN[i]
    }
}

pub fn month(dt: &DateTime<Local>) -> &'static str {
    const EN: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    const AR: [&str; 12] = [
        "يناير",
        "فبراير",
        "مارس",
        "أبريل",
        "مايو",
        "يونيو",
        "يوليو",
        "أغسطس",
        "سبتمبر",
        "أكتوبر",
        "نوفمبر",
        "ديسمبر",
    ];
    let i = dt.month0() as usize;
    if rtl() {
        AR[i]
    } else {
        EN[i]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
