use chrono::{Datelike, Duration, NaiveDate, Weekday};

pub fn parse_weekday(name: &str) -> Weekday {
    match name.to_lowercase().as_str() {
        "monday" => Weekday::Mon,
        "tuesday" => Weekday::Tue,
        "wednesday" => Weekday::Wed,
        "thursday" => Weekday::Thu,
        "friday" => Weekday::Fri,
        "saturday" => Weekday::Sat,
        _ => Weekday::Sun,
    }
}

fn most_recent_on_or_before(date: NaiveDate, weekday: Weekday) -> NaiveDate {
    let mut d = date;
    for _ in 0..7 {
        if d.weekday() == weekday {
            return d;
        }
        d -= Duration::days(1);
    }
    date
}

fn next_on_or_after(date: NaiveDate, weekday: Weekday) -> NaiveDate {
    let mut d = date;
    for _ in 0..7 {
        if d.weekday() == weekday {
            return d;
        }
        d += Duration::days(1);
    }
    date
}

/// Resolves the week containing `reference`, using `start_day`/`end_day` as the two
/// boundary weekdays. The start is the most recent `start_day` on/before `reference`;
/// the end is the next `end_day` after that start — so pairs other than Mon/Sun still
/// resolve to one sensible 7-day-or-less span instead of assuming a fixed offset.
pub fn week_range(reference: NaiveDate, start_day: Weekday, end_day: Weekday) -> (NaiveDate, NaiveDate) {
    let start = most_recent_on_or_before(reference, start_day);
    let end = next_on_or_after(start + Duration::days(1), end_day);
    (start, end)
}

pub fn month_range(reference: NaiveDate) -> (NaiveDate, NaiveDate) {
    let start = NaiveDate::from_ymd_opt(reference.year(), reference.month(), 1).unwrap();
    let next_month_start = if reference.month() == 12 {
        NaiveDate::from_ymd_opt(reference.year() + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(reference.year(), reference.month() + 1, 1).unwrap()
    };
    (start, next_month_start - Duration::days(1))
}

pub fn year_range(reference: NaiveDate) -> (NaiveDate, NaiveDate) {
    let start = NaiveDate::from_ymd_opt(reference.year(), 1, 1).unwrap();
    let end = NaiveDate::from_ymd_opt(reference.year(), 12, 31).unwrap();
    (start, end)
}

/// Converts an inclusive date range into RFC3339 UTC bounds comparable against
/// `started_at` timestamps (which are stored as UTC RFC3339 strings).
pub fn to_rfc3339_bounds(start: NaiveDate, end: NaiveDate) -> (String, String) {
    (
        format!("{}T00:00:00Z", start.format("%Y-%m-%d")),
        format!("{}T23:59:59Z", end.format("%Y-%m-%d")),
    )
}

/// Resolves a period string ("week", "month", or "year") against `reference` into an
/// inclusive date range, shared by both the aggregate Reports view and per-contract
/// timesheet generation. `week_start`/`week_end` only matter for the "week" case — a
/// caller with a fixed Mon-Sun week passes those weekdays directly, while a caller
/// honoring per-client boundaries passes the client's own settings. Any period string
/// other than "week"/"year" resolves to a calendar month, matching this codebase's
/// existing behavior for unrecognized values.
pub fn resolve_period(
    period: &str,
    reference: NaiveDate,
    week_start: Weekday,
    week_end: Weekday,
) -> (NaiveDate, NaiveDate) {
    match period {
        "week" => week_range(reference, week_start, week_end),
        "year" => year_range(reference),
        _ => month_range(reference),
    }
}
