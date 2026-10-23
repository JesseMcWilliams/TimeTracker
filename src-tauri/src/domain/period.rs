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
///
/// The range is anchored to midnight in the user's own timezone, not UTC.
/// `utc_offset_minutes` is their offset east of UTC (UTC-5 passes -300), and the local
/// wall-clock bounds are shifted back by it to land on the right UTC instants.
/// Anchoring at UTC midnight instead — as this did before — files an entry under the
/// neighbouring period whenever local and UTC midnight fall on different dates, e.g. an
/// 8pm Sunday entry in UTC-5 stored as 01:00Z Monday.
pub fn to_rfc3339_bounds(start: NaiveDate, end: NaiveDate, utc_offset_minutes: i32) -> (String, String) {
    let offset = Duration::minutes(utc_offset_minutes as i64);
    let from = start.and_hms_opt(0, 0, 0).expect("midnight is a valid time") - offset;
    let to = end.and_hms_opt(23, 59, 59).expect("23:59:59 is a valid time") - offset;
    (
        format!("{}Z", from.format("%Y-%m-%dT%H:%M:%S")),
        format!("{}Z", to.format("%Y-%m-%dT%H:%M:%S")),
    )
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

    /// Minutes east of UTC for US Eastern Daylight Time, the offset that exposed the
    /// original bug: local midnight there is 04:00Z the same day.
    const UTC_MINUS_4: i32 = -240;

    #[test]
    fn a_zero_offset_anchors_the_range_to_utc_midnight() {
        let start = NaiveDate::from_ymd_opt(2026, 8, 17).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 8, 23).unwrap();

        let (from, to) = to_rfc3339_bounds(start, end, 0);

        assert_eq!(from, "2026-08-17T00:00:00Z");
        assert_eq!(to, "2026-08-23T23:59:59Z");
    }

    #[test]
    fn a_western_offset_shifts_the_range_later_in_utc() {
        let start = NaiveDate::from_ymd_opt(2026, 8, 17).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 8, 23).unwrap();

        let (from, to) = to_rfc3339_bounds(start, end, UTC_MINUS_4);

        assert_eq!(from, "2026-08-17T04:00:00Z", "local midnight in UTC-4 is 04:00Z");
        assert_eq!(to, "2026-08-24T03:59:59Z", "the local day ends after UTC rolls over");
    }

    #[test]
    fn an_eastern_offset_shifts_the_range_earlier_in_utc() {
        let start = NaiveDate::from_ymd_opt(2026, 8, 17).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 8, 23).unwrap();

        let (from, to) = to_rfc3339_bounds(start, end, 120);

        assert_eq!(from, "2026-08-16T22:00:00Z", "local midnight in UTC+2 is 22:00Z the day before");
        assert_eq!(to, "2026-08-23T21:59:59Z");
    }

    /// The regression this change exists for: an entry logged at 8pm local on the last
    /// day of a week lands at 00:00Z the next day, which the old UTC-midnight bounds
    /// pushed out of the period entirely.
    #[test]
    fn a_late_evening_entry_on_the_final_local_day_stays_inside_the_range() {
        let start = NaiveDate::from_ymd_opt(2026, 8, 17).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 8, 23).unwrap();
        let logged_at = "2026-08-24T00:00:00Z"; // 8pm Sunday, UTC-4

        let (from, to) = to_rfc3339_bounds(start, end, UTC_MINUS_4);
        assert!(logged_at >= from.as_str() && logged_at <= to.as_str());

        let (old_from, old_to) = to_rfc3339_bounds(start, end, 0);
        assert!(
            !(logged_at >= old_from.as_str() && logged_at <= old_to.as_str()),
            "UTC-anchored bounds are what dropped this entry from the period"
        );
    }

    /// Offsets that are not whole hours (India is UTC+5:30) must shift by the exact
    /// minute count, not a rounded hour.
    #[test]
    fn a_half_hour_offset_shifts_by_the_exact_minutes() {
        let day = NaiveDate::from_ymd_opt(2026, 8, 17).unwrap();

        let (from, to) = to_rfc3339_bounds(day, day, 330);

        assert_eq!(from, "2026-08-16T18:30:00Z");
        assert_eq!(to, "2026-08-17T18:29:59Z");
    }
}
