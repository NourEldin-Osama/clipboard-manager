//! How times are shown: relative times, day headers and row times.

use super::*;

/// Whole days between the item's local date and today (0 = today).
pub(super) fn days_ago(ts: i64) -> i64 {
    (i18n::local(now()).date_naive() - i18n::local(ts).date_naive()).num_days()
}

pub(super) fn day_month(dt: &chrono::DateTime<chrono::Local>, year: bool) -> String {
    use chrono::Datelike;
    let mut s = format!("{} {}", dt.day(), i18n::month(dt));
    if year {
        s.push_str(&format!(" {}", dt.year()));
    }
    s
}

/// Short time for the edge of a row; the group header already gives the day.
pub(super) fn row_time(ts: i64) -> String {
    use chrono::Datelike;
    let dt = i18n::local(ts);
    match days_ago(ts) {
        0 | 1 => i18n::clock(&dt),
        2..=6 => i18n::weekday(&dt).to_string(),
        _ => day_month(&dt, dt.year() != i18n::local(now()).year()),
    }
}

/// "just now", "5 minutes ago", "3 hours ago", then the short date: card headers
/// have no day groups to lean on.
pub(super) fn ago(ts: i64) -> String {
    let secs = (now() - ts).max(0);
    match secs {
        0..=44 => tr("just now").into(),
        45..=119 => tr("1 minute ago").into(),
        120..=3_599 => trf("{n} minutes ago", &[("n", &(secs / 60))]),
        3_600..=7_199 => tr("1 hour ago").into(),
        7_200..=86_399 => trf("{n} hours ago", &[("n", &(secs / 3_600))]),
        _ => match days_ago(ts) {
            1 => tr("yesterday").into(),
            _ => row_time(ts),
        },
    }
}

/// "Today at 09:59", "Yesterday at 6:20 PM", "Mon 14 Oct 2026 at 10:00".
pub(crate) fn when(ts: i64) -> String {
    let dt = i18n::local(ts);
    let time = i18n::clock(&dt);
    match days_ago(ts) {
        0 => trf("Today at {time}", &[("time", &time)]),
        1 => trf("Yesterday at {time}", &[("time", &time)]),
        _ => trf(
            "{date} at {time}",
            &[
                (
                    "date",
                    &format!("{} {}", i18n::weekday(&dt), day_month(&dt, true)),
                ),
                ("time", &time),
            ],
        ),
    }
}

pub(super) fn group_label(it: &Item, pinned_first: bool) -> &'static str {
    if it.pinned && pinned_first {
        return "Pinned";
    }
    match days_ago(it.created) {
        0 => "Today",
        1 => "Yesterday",
        2..=6 => "This week",
        7..=30 => "This month",
        _ => "Older",
    }
}
