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

/// Converts an inclusive date range into RFC3339 UTC bounds comparable against
/// `started_at` timestamps (which are stored as UTC RFC3339 strings).
pub fn to_rfc3339_bounds(start: NaiveDate, end: NaiveDate) -> (String, String) {
    (
        format!("{}T00:00:00Z", start.format("%Y-%m-%d")),
        format!("{}T23:59:59Z", end.format("%Y-%m-%d")),
    )
}
