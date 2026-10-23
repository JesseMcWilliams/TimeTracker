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

/// Renders an inclusive date range as the two `YYYY-MM-DD` strings that bound a query
/// against `time_entries.local_date`.
///
/// Both sides are plain calendar dates, so the comparison is a string range over an
/// indexed column with no timezone arithmetic at all. Entries carry the day they belong
/// to, decided when they were written (see `time_entries::local_day_of`), which is why
/// this no longer needs to know anything about who is asking or where they are.
pub fn to_local_date_bounds(start: NaiveDate, end: NaiveDate) -> (String, String) {
    (start.format("%Y-%m-%d").to_string(), end.format("%Y-%m-%d").to_string())
}

/// The calendar week (Mon-Sun) immediately before the one containing `reference`.
pub fn previous_week_range(reference: NaiveDate) -> (NaiveDate, NaiveDate) {
    let (current_start, _) = week_range(reference, Weekday::Mon, Weekday::Sun);
    (current_start - Duration::days(7), current_start - Duration::days(1))
}

/// The two calendar weeks (Mon-Sun each) immediately before the one containing
/// `reference`, as one combined span.
pub fn previous_two_weeks_range(reference: NaiveDate) -> (NaiveDate, NaiveDate) {
    let (current_start, _) = week_range(reference, Weekday::Mon, Weekday::Sun);
    (current_start - Duration::days(14), current_start - Duration::days(1))
}

/// The full calendar month immediately before the one containing `reference`.
pub fn previous_month_range(reference: NaiveDate) -> (NaiveDate, NaiveDate) {
    let prev_month_ref = if reference.month() == 1 {
        NaiveDate::from_ymd_opt(reference.year() - 1, 12, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(reference.year(), reference.month() - 1, 1).unwrap()
    };
    month_range(prev_month_ref)
}

/// Resolves one of the email template date-range presets ("previous_week",
/// "previous_2_weeks", "month", "previous_month") against `reference`.
pub fn resolve_date_range_preset(preset: &str, reference: NaiveDate) -> Result<(NaiveDate, NaiveDate), String> {
    match preset {
        "previous_week" => Ok(previous_week_range(reference)),
        "previous_2_weeks" => Ok(previous_two_weeks_range(reference)),
        "month" => Ok(month_range(reference)),
        "previous_month" => Ok(previous_month_range(reference)),
        other => Err(format!("unknown date range preset: {other}")),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_are_the_plain_calendar_dates() {
        let start = NaiveDate::from_ymd_opt(2026, 8, 17).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 8, 23).unwrap();

        let (from, to) = to_local_date_bounds(start, end);

        assert_eq!(from, "2026-08-17");
        assert_eq!(to, "2026-08-23");
    }

    /// The bounds are compared as strings against an indexed TEXT column, so they have
    /// to be zero-padded — "2026-8-7" would sort wrongly against "2026-08-17".
    #[test]
    fn single_digit_months_and_days_are_zero_padded() {
        let day = NaiveDate::from_ymd_opt(2026, 8, 7).unwrap();

        let (from, to) = to_local_date_bounds(day, day);

        assert_eq!(from, "2026-08-07");
        assert_eq!(to, "2026-08-07");
    }

    #[test]
    fn a_single_day_range_has_identical_bounds() {
        let day = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();

        let (from, to) = to_local_date_bounds(day, day);

        assert_eq!(from, to, "an inclusive one-day range covers exactly that day");
    }

    /// String comparison has to agree with calendar order for the BETWEEN to mean what
    /// it reads like, including across month and year boundaries.
    #[test]
    fn bounds_sort_in_calendar_order_as_strings() {
        let dates = [
            NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 1, 9).unwrap(),
            NaiveDate::from_ymd_opt(2026, 1, 10).unwrap(),
            NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
        ];

        let rendered: Vec<String> = dates.iter().map(|d| to_local_date_bounds(*d, *d).0).collect();
        let mut sorted = rendered.clone();
        sorted.sort();

        assert_eq!(rendered, sorted);
    }
}
