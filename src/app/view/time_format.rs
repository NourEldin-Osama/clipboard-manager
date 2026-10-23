//! How times are shown: relative times, day headers and row times.

use super::*;
use crate::store::now;

/// Whole days between the item's local date and today (0 = today).
pub(super) fn days_ago(ts: i64) -> i64 {
    (i18n::local(now()).date_naive() - i18n::local(ts).date_naive()).num_days()
}

pub(super) fn day_month(dt: &chrono::DateTime<chrono::Local>, year: bool) -> String {
    use chrono::Datelike;
    let (day, month) = (dt.day(), i18n::month(dt));
    if year {
        t!(
            "date-day-month-year",
            day = day,
            month = month,
            year = dt.year()
        )
    } else {
        t!("date-day-month", day = day, month = month)
    }
}

/// Short time for the edge of a row; the group header already gives the day.
pub(super) fn row_time(ts: i64) -> String {
    use chrono::Datelike;
    let dt = i18n::local(ts);
    match days_ago(ts) {
        0 | 1 => i18n::clock(&dt),
        2..=6 => i18n::weekday(&dt),
        _ => day_month(&dt, dt.year() != i18n::local(now()).year()),
    }
}

/// "just now", "5 minutes ago", "3 hours ago", then the short date: card headers
/// have no day groups to lean on.
pub(super) fn ago(ts: i64) -> String {
    let secs = (now() - ts).max(0);
    match secs {
        0..=44 => t!("time-just-now"),
        45..=3_599 => t!("time-minutes-ago", count = (secs / 60).max(1)),
        3_600..=86_399 => t!("time-hours-ago", count = secs / 3_600),
        _ => match days_ago(ts) {
            1 => t!("time-yesterday"),
            _ => row_time(ts),
        },
    }
}

/// "Today at 09:59", "Yesterday at 6:20 PM", "Mon 14 Oct 2026 at 10:00".
pub(crate) fn when(ts: i64) -> String {
    let dt = i18n::local(ts);
    let time = i18n::clock(&dt);
    match days_ago(ts) {
        0 => t!("time-today-at", time = time),
        1 => t!("time-yesterday-at", time = time),
        _ => {
            let date = t!(
                "date-weekday-date",
                weekday = i18n::weekday(&dt),
                date = day_month(&dt, true)
            );
            t!("time-date-at", date = date, time = time)
        }
    }
}

/// The day header an item is listed under while browsing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Group {
    Pinned,
    Today,
    Yesterday,
    ThisWeek,
    ThisMonth,
    Older,
}

impl Group {
    pub(super) fn of(it: &Item, pinned_first: bool) -> Self {
        if it.pinned && pinned_first {
            return Group::Pinned;
        }
        match days_ago(it.created) {
            0 => Group::Today,
            1 => Group::Yesterday,
            2..=6 => Group::ThisWeek,
            7..=30 => Group::ThisMonth,
            _ => Group::Older,
        }
    }

    pub(super) fn text(self) -> String {
        match self {
            Group::Pinned => t!("group-pinned"),
            Group::Today => t!("group-today"),
            Group::Yesterday => t!("group-yesterday"),
            Group::ThisWeek => t!("group-this-week"),
            Group::ThisMonth => t!("group-this-month"),
            Group::Older => t!("group-older"),
        }
    }
}
