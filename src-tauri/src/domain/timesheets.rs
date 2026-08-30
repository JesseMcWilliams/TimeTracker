use std::collections::HashSet;
use std::path::Path;

use chrono::{DateTime, Local, NaiveDate, TimeZone, Utc};
use rusqlite::{params, Connection};
use rust_xlsxwriter::{Format, Workbook};
use serde::Serialize;

use super::period::{month_range, parse_weekday, to_rfc3339_bounds, week_range};
use super::DomainResult;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimesheetFile {
    pub path: String,
    pub client_name: String,
    pub contract_name: String,
    pub entry_count: i64,
}

struct ContractInfo {
    id: i64,
    name: String,
    client_name: String,
    week_start: String,
    week_end: String,
    filename_date: String,
}

struct EntryRow {
    started_at: String,
    ended_at: Option<String>,
    duration_secs: Option<i64>,
    rate_snapshot: f64,
    notes: Option<String>,
    category: Option<String>,
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

/// Rounds a stored UTC RFC3339 timestamp to the nearest 5-minute mark — down for a
/// start time, up for an end time (e.g. :07 becomes :05 as a start, :10 as an end) —
/// and formats the result as a local, 24-hour "HH:MM" time (date is shown separately
/// in its own column, so this is deliberately time-only). Rounding is done in UTC
/// epoch-seconds terms, which is equivalent to rounding the local wall-clock minute
/// since every real-world timezone offset is itself a whole number of minutes.
fn local_time_hm_rounded(rfc3339: &str, round_up: bool) -> String {
    const INTERVAL_SECS: i64 = 5 * 60;
    let Ok(dt) = DateTime::parse_from_rfc3339(rfc3339) else {
        return String::new();
    };
    let secs = dt.timestamp();
    let rounded_secs = if round_up {
        secs.div_euclid(INTERVAL_SECS) * INTERVAL_SECS
            + if secs.rem_euclid(INTERVAL_SECS) == 0 { 0 } else { INTERVAL_SECS }
    } else {
        secs.div_euclid(INTERVAL_SECS) * INTERVAL_SECS
    };
    match Utc.timestamp_opt(rounded_secs, 0) {
        chrono::LocalResult::Single(rounded) => rounded.with_timezone(&Local).format("%H:%M").to_string(),
        _ => String::new(),
    }
}

/// Formats a stored UTC RFC3339 timestamp as its LOCAL calendar date. Using the raw
/// UTC date substring here (instead of converting timezones first) was the cause of a
/// reported bug: an entry that's 8/24 in local time but past midnight UTC (8/25) showed
/// the correct local start/end time but the wrong, UTC-derived date.
fn local_date(rfc3339: &str) -> String {
    DateTime::parse_from_rfc3339(rfc3339)
        .map(|dt| dt.with_timezone(&Local).format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

fn duration_hm(secs: i64) -> String {
    format!("{}:{:02}", secs / 3600, (secs % 3600) / 60)
}

/// Generates one .xlsx timesheet per contract that has entries in the resolved period,
/// named `{date}_{client}_{contract}_{yourFullName}.xlsx`, where `{date}` is either
/// the first or last day of that contract's resolved period depending on its own
/// `filename_date` setting ("start" or "end"; "end" — the last day — is the default).
/// For "week" periods, each contract's range is computed from its own client's
/// `week_start`/`week_end` (so a client billed Sun-Sat gets a Sun-Sat sheet even if
/// another client uses Mon-Sun); "month" periods use the same calendar month for
/// everyone. Contracts with no entries in their resolved period are skipped rather
/// than producing an empty file. Rate and Amount columns are only included when
/// `include_rate_amount` is set — by default the sheet is just
/// Date/Start/End/Hours/Category/Notes, with Start/End times rounded to the nearest 5
/// minutes (down for start, up for end) for readability. `contract_id` restricts
/// output to a single contract; otherwise `client_id` restricts to that client's
/// contracts; if neither is set, every active contract is considered (contract_id
/// wins if both are set).
pub fn generate_timesheets(
    conn: &Connection,
    period: &str,
    reference_date: &str,
    output_folder: &str,
    user_full_name: &str,
    include_rate_amount: bool,
    client_id: Option<i64>,
    contract_id: Option<i64>,
) -> DomainResult<Vec<TimesheetFile>> {
    let reference: NaiveDate = reference_date.parse().map_err(|e| format!("invalid date: {e}"))?;

    let mut sql = "SELECT c.id, c.name, cl.name, cl.week_start, cl.week_end, c.filename_date
         FROM contracts c JOIN clients cl ON cl.id = c.client_id
         WHERE c.archived_at IS NULL"
        .to_string();
    let mut sql_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(id) = contract_id {
        sql.push_str(" AND c.id = ?");
        sql_params.push(Box::new(id));
    } else if let Some(id) = client_id {
        sql.push_str(" AND c.client_id = ?");
        sql_params.push(Box::new(id));
    }

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = sql_params.iter().map(|p| p.as_ref()).collect();
    let contracts: Vec<ContractInfo> = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok(ContractInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                client_name: row.get(2)?,
                week_start: row.get(3)?,
                week_end: row.get(4)?,
                filename_date: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut used_names: HashSet<String> = HashSet::new();
    let mut results = Vec::new();

    for contract in contracts {
        let (start, end) = if period == "week" {
            week_range(reference, parse_weekday(&contract.week_start), parse_weekday(&contract.week_end))
        } else {
            month_range(reference)
        };
        let (from, to) = to_rfc3339_bounds(start, end);

        let mut entry_stmt = conn
            .prepare(
                "SELECT te.started_at, te.ended_at, te.duration_secs, te.rate_snapshot, te.notes, tc.code
                 FROM time_entries te LEFT JOIN tracking_codes tc ON tc.id = te.tracking_code_id
                 WHERE te.contract_id = ?1 AND te.deleted_at IS NULL
                   AND te.started_at >= ?2 AND te.started_at <= ?3
                 ORDER BY te.started_at",
            )
            .map_err(|e| e.to_string())?;

        let entries: Vec<EntryRow> = entry_stmt
            .query_map(params![contract.id, from, to], |row| {
                Ok(EntryRow {
                    started_at: row.get(0)?,
                    ended_at: row.get(1)?,
                    duration_secs: row.get(2)?,
                    rate_snapshot: row.get(3)?,
                    notes: row.get(4)?,
                    category: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        if entries.is_empty() {
            continue;
        }

        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();
        let header_format = Format::new().set_bold();
        let money_format = Format::new().set_num_format("#,##0.00");
        let money_bold_format = Format::new().set_num_format("#,##0.00").set_bold();

        let mut headers = vec!["Date", "Start Time", "End Time", "HH:MM", "Category", "Notes"];
        if include_rate_amount {
            headers.push("Rate");
            headers.push("Amount");
        }
        for (col, header) in headers.iter().enumerate() {
            worksheet
                .write_with_format(0, col as u16, *header, &header_format)
                .map_err(|e| e.to_string())?;
        }

        let column_widths: [f64; 8] = [12.0, 11.0, 11.0, 8.0, 18.0, 40.0, 10.0, 12.0];
        for (col, width) in column_widths.iter().enumerate().take(headers.len()) {
            worksheet.set_column_width(col as u16, *width).map_err(|e| e.to_string())?;
        }

        let mut total_secs = 0i64;
        let mut total_amount = 0f64;

        for (i, entry) in entries.iter().enumerate() {
            let row = (i + 1) as u32;
            let secs = entry.duration_secs.unwrap_or(0);
            let amount = (secs as f64 / 3600.0) * entry.rate_snapshot;
            total_secs += secs;
            total_amount += amount;

            worksheet.write(row, 0, local_date(&entry.started_at)).map_err(|e| e.to_string())?;
            worksheet
                .write(row, 1, local_time_hm_rounded(&entry.started_at, false))
                .map_err(|e| e.to_string())?;
            worksheet
                .write(
                    row,
                    2,
                    entry.ended_at.as_deref().map(|e| local_time_hm_rounded(e, true)).unwrap_or_default(),
                )
                .map_err(|e| e.to_string())?;
            worksheet.write(row, 3, duration_hm(secs)).map_err(|e| e.to_string())?;
            worksheet
                .write(row, 4, entry.category.as_deref().unwrap_or(""))
                .map_err(|e| e.to_string())?;
            worksheet
                .write(row, 5, entry.notes.as_deref().unwrap_or(""))
                .map_err(|e| e.to_string())?;
            if include_rate_amount {
                worksheet
                    .write_with_format(row, 6, entry.rate_snapshot, &money_format)
                    .map_err(|e| e.to_string())?;
                worksheet.write_with_format(row, 7, amount, &money_format).map_err(|e| e.to_string())?;
            }
        }

        let total_row = (entries.len() + 1) as u32;
        worksheet
            .write_with_format(total_row, 2, "Total", &header_format)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_with_format(total_row, 3, duration_hm(total_secs), &header_format)
            .map_err(|e| e.to_string())?;
        if include_rate_amount {
            worksheet
                .write_with_format(total_row, 7, total_amount, &money_bold_format)
                .map_err(|e| e.to_string())?;
        }

        let filename_date = if contract.filename_date == "start" { start } else { end };
        let base_name = format!(
            "{}_{}_{}_{}",
            filename_date.format("%Y-%m-%d"),
            sanitize(&contract.client_name),
            sanitize(&contract.name),
            sanitize(user_full_name)
        );
        let mut filename = format!("{base_name}.xlsx");
        let mut suffix = 2;
        while used_names.contains(&filename) {
            filename = format!("{base_name}_{suffix}.xlsx");
            suffix += 1;
        }
        used_names.insert(filename.clone());

        let path = Path::new(output_folder).join(&filename);
        workbook.save(&path).map_err(|e| e.to_string())?;

        results.push(TimesheetFile {
            path: path.to_string_lossy().to_string(),
            client_name: contract.client_name,
            contract_name: contract.name,
            entry_count: entries.len() as i64,
        });
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds an RFC3339 timestamp for the given LOCAL wall-clock time, so these tests
    /// pass regardless of the machine's configured timezone (rounding is verified
    /// entirely in terms of local hour/minute, never assuming Local == UTC).
    fn local_rfc3339(hour: u32, minute: u32, second: u32) -> String {
        Local.with_ymd_and_hms(2026, 8, 24, hour, minute, second).unwrap().to_rfc3339()
    }

    #[test]
    fn start_times_round_down_to_the_nearest_5_minutes() {
        assert_eq!(local_time_hm_rounded(&local_rfc3339(7, 7, 0), false), "07:05");
        assert_eq!(local_time_hm_rounded(&local_rfc3339(7, 9, 59), false), "07:05");
        // Already on a 5-minute mark: stays put.
        assert_eq!(local_time_hm_rounded(&local_rfc3339(7, 5, 0), false), "07:05");
        assert_eq!(local_time_hm_rounded(&local_rfc3339(7, 0, 30), false), "07:00");
    }

    #[test]
    fn end_times_round_up_to_the_nearest_5_minutes() {
        assert_eq!(local_time_hm_rounded(&local_rfc3339(7, 7, 0), true), "07:10");
        assert_eq!(local_time_hm_rounded(&local_rfc3339(7, 1, 0), true), "07:05");
        // Already on a 5-minute mark: stays put (does not bump to the next mark).
        assert_eq!(local_time_hm_rounded(&local_rfc3339(7, 5, 0), true), "07:05");
        // Rounding up correctly rolls over an hour boundary.
        assert_eq!(local_time_hm_rounded(&local_rfc3339(7, 57, 0), true), "08:00");
    }
}
