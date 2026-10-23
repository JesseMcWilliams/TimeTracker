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

/// The week containing `reference`, starting on `week_start`.
pub fn current_week_range(reference: NaiveDate, week_start: Weekday) -> (NaiveDate, NaiveDate) {
    let start = week_range(reference, week_start, week_start.pred()).0;
    (start, start + Duration::days(6))
}

/// The week immediately before the one containing `reference`.
pub fn previous_week_range(reference: NaiveDate, week_start: Weekday) -> (NaiveDate, NaiveDate) {
    let (current_start, _) = current_week_range(reference, week_start);
    (current_start - Duration::days(7), current_start - Duration::days(1))
}

/// The week containing `reference` together with the one before it.
pub fn current_two_weeks_range(reference: NaiveDate, week_start: Weekday) -> (NaiveDate, NaiveDate) {
    let (current_start, current_end) = current_week_range(reference, week_start);
    (current_start - Duration::days(7), current_end)
}

/// The two weeks immediately before the one containing `reference`, as one span.
pub fn previous_two_weeks_range(reference: NaiveDate, week_start: Weekday) -> (NaiveDate, NaiveDate) {
    let (current_start, _) = current_week_range(reference, week_start);
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

/// Resolves one of the email template date-range presets against `reference`.
///
/// `week_start` is the weekday a pay week begins on, taken from the user's profile —
/// the email spans every client, so there is no single client's week to use. The
/// week-based presets honour it; the month-based ones ignore it.
pub fn resolve_date_range_preset(
    preset: &str,
    reference: NaiveDate,
    week_start: Weekday,
) -> Result<(NaiveDate, NaiveDate), String> {
    match preset {
        "current_week" => Ok(current_week_range(reference, week_start)),
        "previous_week" => Ok(previous_week_range(reference, week_start)),
        "current_2_weeks" => Ok(current_two_weeks_range(reference, week_start)),
        "previous_2_weeks" => Ok(previous_two_weeks_range(reference, week_start)),
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


    /// 2026-10-23 is a Friday; its Monday-start week is 10-19..10-25 and the one
    /// before is 10-12..10-18.
    const FRIDAY: (i32, u32, u32) = (2026, 10, 23);

    fn friday() -> NaiveDate {
        NaiveDate::from_ymd_opt(FRIDAY.0, FRIDAY.1, FRIDAY.2).unwrap()
    }

    fn span(range: (NaiveDate, NaiveDate)) -> (String, String) {
        (range.0.format("%Y-%m-%d").to_string(), range.1.format("%Y-%m-%d").to_string())
    }

    /// The gap this preset was added to close: run mid-week, "previous week" reports
    /// on the week *before* the one you are in, which is not the week you are asking
    /// about when you send the email on a Friday.
    #[test]
    fn current_week_covers_the_week_in_progress_not_the_one_before() {
        let current = span(resolve_date_range_preset("current_week", friday(), Weekday::Mon).unwrap());
        let previous = span(resolve_date_range_preset("previous_week", friday(), Weekday::Mon).unwrap());

        assert_eq!(current, ("2026-10-19".into(), "2026-10-25".into()));
        assert_eq!(previous, ("2026-10-12".into(), "2026-10-18".into()));
    }

    #[test]
    fn a_week_is_seven_days_whichever_day_it_starts_on() {
        for start in [Weekday::Mon, Weekday::Sun, Weekday::Wed, Weekday::Sat] {
            let (from, to) = current_week_range(friday(), start);
            assert_eq!(from.weekday(), start, "the week opens on its start day");
            assert_eq!((to - from).num_days(), 6, "inclusive seven-day span");
            assert!(from <= friday() && friday() <= to, "the reference date falls inside its own week");
        }
    }

    #[test]
    fn the_week_presets_honour_a_non_monday_start() {
        // A Sunday-start week containing Friday 10-23 runs 10-18..10-24.
        assert_eq!(
            span(resolve_date_range_preset("current_week", friday(), Weekday::Sun).unwrap()),
            ("2026-10-18".into(), "2026-10-24".into())
        );
        assert_eq!(
            span(resolve_date_range_preset("previous_week", friday(), Weekday::Sun).unwrap()),
            ("2026-10-11".into(), "2026-10-17".into())
        );
    }

    #[test]
    fn the_two_week_presets_span_fourteen_days_and_do_not_overlap_each_other() {
        let current = resolve_date_range_preset("current_2_weeks", friday(), Weekday::Mon).unwrap();
        let previous = resolve_date_range_preset("previous_2_weeks", friday(), Weekday::Mon).unwrap();

        assert_eq!(span(current), ("2026-10-12".into(), "2026-10-25".into()));
        assert_eq!(span(previous), ("2026-10-05".into(), "2026-10-18".into()));
        assert_eq!((current.1 - current.0).num_days(), 13);
        assert_eq!((previous.1 - previous.0).num_days(), 13);
    }

    /// previous_* must stop before the current week opens, or a period would be
    /// reported on twice.
    #[test]
    fn the_previous_presets_end_before_the_current_week_begins() {
        let (current_start, _) = current_week_range(friday(), Weekday::Mon);
        for preset in ["previous_week", "previous_2_weeks"] {
            let (_, end) = resolve_date_range_preset(preset, friday(), Weekday::Mon).unwrap();
            assert!(end < current_start, "{preset} overlaps the current week");
        }
    }

    #[test]
    fn the_month_presets_ignore_the_week_start() {
        for start in [Weekday::Mon, Weekday::Sun] {
            assert_eq!(
                span(resolve_date_range_preset("month", friday(), start).unwrap()),
                ("2026-10-01".into(), "2026-10-31".into())
            );
            assert_eq!(
                span(resolve_date_range_preset("previous_month", friday(), start).unwrap()),
                ("2026-09-01".into(), "2026-09-30".into())
            );
        }
    }

    #[test]
    fn an_unknown_preset_is_rejected_rather_than_guessed_at() {
        assert!(resolve_date_range_preset("last_fortnight", friday(), Weekday::Mon).is_err());
    }

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
