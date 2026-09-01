use std::collections::HashSet;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, NaiveDate, TimeZone, Utc};
use rusqlite::{params, Connection};
use rust_xlsxwriter::{Format, Workbook};
use serde::Serialize;

use super::period::{parse_weekday, resolve_period, to_rfc3339_bounds};
use super::DomainResult;

/// Which file format "Create Timesheet" writes, driven by the user profile's Output
/// type setting rather than always writing .xlsx.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimesheetFormat {
    Xlsx,
    Csv,
}

impl TimesheetFormat {
    fn from_output_type(output_type: &str) -> Self {
        if output_type.eq_ignore_ascii_case("csv") {
            TimesheetFormat::Csv
        } else {
            TimesheetFormat::Xlsx
        }
    }

    fn extension(self) -> &'static str {
        match self {
            TimesheetFormat::Xlsx => "xlsx",
            TimesheetFormat::Csv => "csv",
        }
    }
}

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

fn fetch_active_contracts(
    conn: &Connection,
    contract_id: Option<i64>,
    client_id: Option<i64>,
) -> DomainResult<Vec<ContractInfo>> {
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
    let rows = stmt
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
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn fetch_contract_entries(conn: &Connection, contract_id: i64, from: &str, to: &str) -> DomainResult<Vec<EntryRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT te.started_at, te.ended_at, te.duration_secs, te.rate_snapshot, te.notes, tc.code
             FROM time_entries te LEFT JOIN tracking_codes tc ON tc.id = te.tracking_code_id
             WHERE te.contract_id = ?1 AND te.deleted_at IS NULL
               AND te.started_at >= ?2 AND te.started_at <= ?3
             ORDER BY te.started_at",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![contract_id, from, to], |row| {
            Ok(EntryRow {
                started_at: row.get(0)?,
                ended_at: row.get(1)?,
                duration_secs: row.get(2)?,
                rate_snapshot: row.get(3)?,
                notes: row.get(4)?,
                category: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn unique_filename(base_name: &str, ext: &str, used_names: &mut HashSet<String>) -> String {
    let mut filename = format!("{base_name}.{ext}");
    let mut suffix = 2;
    while used_names.contains(&filename) {
        filename = format!("{base_name}_{suffix}.{ext}");
        suffix += 1;
    }
    used_names.insert(filename.clone());
    filename
}

/// Flattens every contract's entries into one chronologically-sorted list. When there's
/// more than one contract group (a client-scoped timesheet spanning several contracts),
/// rows from every contract are interleaved by start time — rather than grouped
/// contract-by-contract — so the sheet/file reads as one continuous day-by-day log.
fn sorted_entries(groups: Vec<(String, Vec<EntryRow>)>) -> Vec<(String, EntryRow)> {
    let mut all: Vec<(String, EntryRow)> = groups
        .into_iter()
        .flat_map(|(name, entries)| entries.into_iter().map(move |e| (name.clone(), e)))
        .collect();
    all.sort_by(|a, b| a.1.started_at.cmp(&b.1.started_at));
    all
}

/// Writes one or more contracts' already-sorted entries to `output_folder/filename` in
/// the given format. Returns the saved path and the total entry count written.
fn write_timesheet(
    output_folder: &str,
    filename: &str,
    include_rate_amount: bool,
    format: TimesheetFormat,
    groups: Vec<(String, Vec<EntryRow>)>,
) -> DomainResult<(PathBuf, i64)> {
    let all = sorted_entries(groups);
    let count = all.len() as i64;
    let path = match format {
        TimesheetFormat::Xlsx => write_timesheet_xlsx(output_folder, filename, include_rate_amount, &all)?,
        TimesheetFormat::Csv => write_timesheet_csv(output_folder, filename, include_rate_amount, &all)?,
    };
    Ok((path, count))
}

/// Every sheet gets a "Contract" column (even when there's only one, for a consistent
/// column layout across every exported timesheet).
fn write_timesheet_xlsx(
    output_folder: &str,
    filename: &str,
    include_rate_amount: bool,
    all: &[(String, EntryRow)],
) -> DomainResult<PathBuf> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    let header_format = Format::new().set_bold();
    let money_format = Format::new().set_num_format("#,##0.00");
    let money_bold_format = Format::new().set_num_format("#,##0.00").set_bold();

    let mut headers: Vec<&str> = vec!["Date", "Contract", "Start Time", "End Time", "HH:MM", "Category", "Notes"];
    if include_rate_amount {
        headers.push("Rate");
        headers.push("Amount");
    }
    for (col, header) in headers.iter().enumerate() {
        worksheet
            .write_with_format(0, col as u16, *header, &header_format)
            .map_err(|e| e.to_string())?;
    }

    let mut widths: Vec<f64> = vec![12.0, 18.0, 11.0, 11.0, 8.0, 18.0, 40.0];
    if include_rate_amount {
        widths.extend_from_slice(&[10.0, 12.0]);
    }
    for (col, width) in widths.iter().enumerate() {
        worksheet.set_column_width(col as u16, *width).map_err(|e| e.to_string())?;
    }

    // Computing these once keeps every write/total-row reference correct without
    // repeating the column layout everywhere.
    let base: u16 = 2;
    let col_start = base;
    let col_end = base + 1;
    let col_hhmm = base + 2;
    let col_category = base + 3;
    let col_notes = base + 4;
    let col_rate = base + 5;
    let col_amount = base + 6;

    let mut total_secs = 0i64;
    let mut total_amount = 0f64;

    for (i, (contract_name, entry)) in all.iter().enumerate() {
        let row = (i + 1) as u32;
        let secs = entry.duration_secs.unwrap_or(0);
        let amount = (secs as f64 / 3600.0) * entry.rate_snapshot;
        total_secs += secs;
        total_amount += amount;

        worksheet.write(row, 0, local_date(&entry.started_at)).map_err(|e| e.to_string())?;
        worksheet.write(row, 1, contract_name.as_str()).map_err(|e| e.to_string())?;
        worksheet
            .write(row, col_start, local_time_hm_rounded(&entry.started_at, false))
            .map_err(|e| e.to_string())?;
        worksheet
            .write(
                row,
                col_end,
                entry.ended_at.as_deref().map(|e| local_time_hm_rounded(e, true)).unwrap_or_default(),
            )
            .map_err(|e| e.to_string())?;
        worksheet.write(row, col_hhmm, duration_hm(secs)).map_err(|e| e.to_string())?;
        worksheet
            .write(row, col_category, entry.category.as_deref().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        worksheet
            .write(row, col_notes, entry.notes.as_deref().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        if include_rate_amount {
            worksheet
                .write_with_format(row, col_rate, entry.rate_snapshot, &money_format)
                .map_err(|e| e.to_string())?;
            worksheet.write_with_format(row, col_amount, amount, &money_format).map_err(|e| e.to_string())?;
        }
    }

    let total_row = (all.len() + 1) as u32;
    worksheet
        .write_with_format(total_row, col_end, "Total", &header_format)
        .map_err(|e| e.to_string())?;
    worksheet
        .write_with_format(total_row, col_hhmm, duration_hm(total_secs), &header_format)
        .map_err(|e| e.to_string())?;
    if include_rate_amount {
        // Summed across every entry regardless of contract — correct as long as a
        // client's contracts share one currency (the common case); mixed-currency
        // clients would see a numerically meaningless combined total here.
        worksheet
            .write_with_format(total_row, col_amount, total_amount, &money_bold_format)
            .map_err(|e| e.to_string())?;
    }

    let path = Path::new(output_folder).join(filename);
    workbook.save(&path).map_err(|e| e.to_string())?;

    Ok(path)
}

/// The .csv equivalent of `write_timesheet_xlsx` — same columns, same row order, same
/// bolded-in-spirit "Total" row (CSV has no cell formatting, so the total row is just
/// plain text/numbers like every other row) — for users whose Output type profile
/// setting is "csv" rather than "xlsx".
fn write_timesheet_csv(
    output_folder: &str,
    filename: &str,
    include_rate_amount: bool,
    all: &[(String, EntryRow)],
) -> DomainResult<PathBuf> {
    let path = Path::new(output_folder).join(filename);
    let mut writer = csv::Writer::from_path(&path).map_err(|e| e.to_string())?;

    let mut headers: Vec<&str> = vec!["Date", "Contract", "Start Time", "End Time", "HH:MM", "Category", "Notes"];
    if include_rate_amount {
        headers.push("Rate");
        headers.push("Amount");
    }
    writer.write_record(&headers).map_err(|e| e.to_string())?;

    let mut total_secs = 0i64;
    let mut total_amount = 0f64;

    for (contract_name, entry) in all {
        let secs = entry.duration_secs.unwrap_or(0);
        let amount = (secs as f64 / 3600.0) * entry.rate_snapshot;
        total_secs += secs;
        total_amount += amount;

        let mut record = vec![
            local_date(&entry.started_at),
            contract_name.clone(),
            local_time_hm_rounded(&entry.started_at, false),
            entry.ended_at.as_deref().map(|e| local_time_hm_rounded(e, true)).unwrap_or_default(),
            duration_hm(secs),
            entry.category.clone().unwrap_or_default(),
            entry.notes.clone().unwrap_or_default(),
        ];
        if include_rate_amount {
            record.push(format!("{:.2}", entry.rate_snapshot));
            record.push(format!("{amount:.2}"));
        }
        writer.write_record(&record).map_err(|e| e.to_string())?;
    }

    let mut total_record =
        vec![String::new(), String::new(), String::new(), "Total".to_string(), duration_hm(total_secs), String::new(), String::new()];
    if include_rate_amount {
        // Summed across every entry regardless of contract — correct as long as a
        // client's contracts share one currency (the common case); mixed-currency
        // clients would see a numerically meaningless combined total here — same
        // caveat as the .xlsx total row.
        total_record.push(String::new());
        total_record.push(format!("{total_amount:.2}"));
    }
    writer.write_record(&total_record).map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())?;

    Ok(path)
}

fn fetch_client_ids_with_active_contracts(conn: &Connection) -> DomainResult<Vec<i64>> {
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT cl.id FROM contracts c JOIN clients cl ON cl.id = c.client_id
             WHERE c.archived_at IS NULL ORDER BY cl.name",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| row.get::<_, i64>(0)).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Generates a timesheet in the format named by `output_type` ("xlsx" or "csv" — the
/// user profile's Output type setting; anything else falls back to xlsx). Scoped to a
/// single `contract_id`, writes one file for just that contract, named
/// `{date}_{client}_{contract}_{yourFullName}.{ext}`. Otherwise (no `contract_id`),
/// writes one COMBINED file per client — either just the one `client_id` given, or
/// every client that has an active contract — covering all of that client's contracts
/// in a single sheet/file (see `generate_combined_client_timesheet`). Every
/// sheet/file, single-contract or combined, has a Contract column identifying each
/// row. `{date}` is either the first or last day of the resolved period, per each
/// contract's own `filename_date` setting ("start"/"end"; "end" — the last day — is
/// the default; for a multi-contract client, "start" is only used if every one of its
/// contracts agrees). For "week" periods, the range is computed from the relevant
/// client's own `week_start`/`week_end`; "month" periods use the same calendar month
/// for everyone; "year" periods use the same calendar year for everyone (see
/// `period::resolve_period`). A client/contract with no entries in its resolved
/// period is skipped rather than producing an empty file. Rate and Amount columns are
/// only included when `include_rate_amount` is set, with Start/End times rounded to
/// the nearest 5 minutes (down for start, up for end) for readability.
#[allow(clippy::too_many_arguments)]
pub fn generate_timesheets(
    conn: &Connection,
    period: &str,
    reference_date: &str,
    output_folder: &str,
    user_full_name: &str,
    include_rate_amount: bool,
    client_id: Option<i64>,
    contract_id: Option<i64>,
    output_type: &str,
) -> DomainResult<Vec<TimesheetFile>> {
    let reference: NaiveDate = reference_date.parse().map_err(|e| format!("invalid date: {e}"))?;
    let format = TimesheetFormat::from_output_type(output_type);

    if let Some(contract_id) = contract_id {
        return generate_single_contract_timesheet(
            conn,
            period,
            reference,
            output_folder,
            user_full_name,
            include_rate_amount,
            contract_id,
            format,
        );
    }

    let client_ids = match client_id {
        Some(id) => vec![id],
        None => fetch_client_ids_with_active_contracts(conn)?,
    };

    let mut used_names: HashSet<String> = HashSet::new();
    let mut results = Vec::new();
    for id in client_ids {
        results.extend(generate_combined_client_timesheet(
            conn,
            period,
            reference,
            output_folder,
            user_full_name,
            include_rate_amount,
            id,
            &mut used_names,
            format,
        )?);
    }
    Ok(results)
}

#[allow(clippy::too_many_arguments)]
fn generate_single_contract_timesheet(
    conn: &Connection,
    period: &str,
    reference: NaiveDate,
    output_folder: &str,
    user_full_name: &str,
    include_rate_amount: bool,
    contract_id: i64,
    format: TimesheetFormat,
) -> DomainResult<Vec<TimesheetFile>> {
    let Some(contract) = fetch_active_contracts(conn, Some(contract_id), None)?.into_iter().next() else {
        return Ok(Vec::new());
    };

    let (start, end) =
        resolve_period(period, reference, parse_weekday(&contract.week_start), parse_weekday(&contract.week_end));
    let (from, to) = to_rfc3339_bounds(start, end);
    let entries = fetch_contract_entries(conn, contract.id, &from, &to)?;
    if entries.is_empty() {
        return Ok(Vec::new());
    }

    let filename_date = if contract.filename_date == "start" { start } else { end };
    let base_name = format!(
        "{}_{}_{}_{}",
        filename_date.format("%Y-%m-%d"),
        sanitize(&contract.client_name),
        sanitize(&contract.name),
        sanitize(user_full_name)
    );
    let filename = format!("{base_name}.{}", format.extension());

    let (path, entry_count) = write_timesheet(
        output_folder,
        &filename,
        include_rate_amount,
        format,
        vec![(contract.name.clone(), entries)],
    )?;

    Ok(vec![TimesheetFile {
        path: path.to_string_lossy().to_string(),
        client_name: contract.client_name,
        contract_name: contract.name,
        entry_count,
    }])
}

/// Combines every one of a client's active contracts into a single timesheet, with a
/// Contract column identifying each row's contract (present regardless of how many
/// contracts end up with entries — see `write_timesheet_xlsx`/`write_timesheet_csv`).
/// Every contract under one client shares that client's `week_start`/`week_end`, so
/// the resolved "week" period is identical for all of them — computed once, not per
/// contract. Skips (does not write) any contract with no entries in the period;
/// returns an empty result rather than an empty file if none of the client's
/// contracts have any. `used_names` is shared across a whole `generate_timesheets`
/// call so that, in the unlikely case two different clients' names sanitize to the
/// same filename, the second gets a numeric suffix instead of silently overwriting
/// the first.
#[allow(clippy::too_many_arguments)]
fn generate_combined_client_timesheet(
    conn: &Connection,
    period: &str,
    reference: NaiveDate,
    output_folder: &str,
    user_full_name: &str,
    include_rate_amount: bool,
    client_id: i64,
    used_names: &mut HashSet<String>,
    format: TimesheetFormat,
) -> DomainResult<Vec<TimesheetFile>> {
    let contracts = fetch_active_contracts(conn, None, Some(client_id))?;
    let Some(first) = contracts.first() else {
        return Ok(Vec::new());
    };
    let client_name = first.client_name.clone();

    let (start, end) =
        resolve_period(period, reference, parse_weekday(&first.week_start), parse_weekday(&first.week_end));
    let (from, to) = to_rfc3339_bounds(start, end);

    let mut groups = Vec::new();
    for contract in &contracts {
        let entries = fetch_contract_entries(conn, contract.id, &from, &to)?;
        if !entries.is_empty() {
            groups.push((contract.name.clone(), entries));
        }
    }
    if groups.is_empty() {
        return Ok(Vec::new());
    }

    // Only use the period's start date in the filename if every one of this
    // client's contracts explicitly prefers it; otherwise (mixed settings, or all
    // "end") default to the last day, matching the system-wide default.
    let all_prefer_start = contracts.iter().all(|c| c.filename_date == "start");
    let filename_date = if all_prefer_start { start } else { end };

    let base_name =
        format!("{}_{}_{}", filename_date.format("%Y-%m-%d"), sanitize(&client_name), sanitize(user_full_name));
    let filename = unique_filename(&base_name, format.extension(), used_names);
    let contract_name = if groups.len() == 1 { groups[0].0.clone() } else { "All Contracts".to_string() };

    let (path, entry_count) = write_timesheet(output_folder, &filename, include_rate_amount, format, groups)?;

    Ok(vec![TimesheetFile {
        path: path.to_string_lossy().to_string(),
        client_name,
        contract_name,
        entry_count,
    }])
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

    fn cell_str(cell: &calamine::Data) -> String {
        match cell {
            calamine::Data::String(s) => s.clone(),
            other => format!("{other:?}"),
        }
    }

    #[test]
    fn client_scoped_timesheet_combines_every_contract_into_one_sheet() {
        use crate::domain::contracts::{create_client, create_contract};
        use crate::domain::time_entries::create_manual_entry;
        use calamine::{open_workbook_auto, Reader};
        use std::fs;

        let pid = std::process::id();
        let db_path = std::env::temp_dir().join(format!("timetracker_test_timesheets_{pid}_combined.sqlite"));
        let _ = fs::remove_file(&db_path);
        let conn = crate::db::open(&db_path).expect("open db");
        let out_dir = std::env::temp_dir().join(format!("timetracker_test_timesheets_{pid}_out"));
        let _ = fs::remove_dir_all(&out_dir);
        fs::create_dir_all(&out_dir).expect("create output dir");

        let client_id = create_client(&conn, "Combined Client", None).expect("create client");
        let contract_a = create_contract(&conn, client_id, "Contract A", "USD", 50.0).expect("create contract a");
        let contract_b = create_contract(&conn, client_id, "Contract B", "USD", 75.0).expect("create contract b");

        create_manual_entry(&conn, contract_a, "2026-08-24T09:00:00Z", "2026-08-24T10:00:00Z", None, None)
            .expect("create entry a");
        create_manual_entry(&conn, contract_b, "2026-08-25T13:00:00Z", "2026-08-25T14:00:00Z", None, None)
            .expect("create entry b");

        let files = generate_timesheets(
            &conn,
            "week",
            "2026-08-24",
            out_dir.to_str().unwrap(),
            "Jane Consultant",
            false,
            Some(client_id),
            None,
            "xlsx",
        )
        .expect("generate timesheets");

        assert_eq!(files.len(), 1, "one combined file for the whole client, not one per contract");
        assert_eq!(files[0].contract_name, "All Contracts");
        assert_eq!(files[0].entry_count, 2);
        assert!(!Path::new(&files[0].path).file_name().unwrap().to_string_lossy().contains("Contract"));

        let mut workbook = open_workbook_auto(&files[0].path).expect("open generated workbook");
        let sheet_name = workbook.sheet_names()[0].clone();
        let range = workbook.worksheet_range(&sheet_name).expect("read sheet");
        let mut rows = range.rows();

        let header: Vec<String> = rows.next().expect("header row").iter().map(cell_str).collect();
        assert!(header.contains(&"Contract".to_string()), "combined sheet must have a Contract column: {header:?}");
        let contract_col = header.iter().position(|h| h == "Contract").unwrap();

        let contract_names: Vec<String> = rows.take(2).map(|row| cell_str(&row[contract_col])).collect();
        assert!(contract_names.contains(&"Contract A".to_string()), "{contract_names:?}");
        assert!(contract_names.contains(&"Contract B".to_string()), "{contract_names:?}");

        let _ = fs::remove_file(&db_path);
        let _ = fs::remove_dir_all(&out_dir);
    }

    #[test]
    fn contract_scoped_timesheet_still_has_a_contract_column() {
        use crate::domain::contracts::{create_client, create_contract};
        use crate::domain::time_entries::create_manual_entry;
        use calamine::{open_workbook_auto, Reader};
        use std::fs;

        let pid = std::process::id();
        let db_path = std::env::temp_dir().join(format!("timetracker_test_timesheets_{pid}_single.sqlite"));
        let _ = fs::remove_file(&db_path);
        let conn = crate::db::open(&db_path).expect("open db");
        let out_dir = std::env::temp_dir().join(format!("timetracker_test_timesheets_{pid}_single_out"));
        let _ = fs::remove_dir_all(&out_dir);
        fs::create_dir_all(&out_dir).expect("create output dir");

        let client_id = create_client(&conn, "Solo Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Only Contract", "USD", 50.0).expect("create contract");
        create_manual_entry(&conn, contract_id, "2026-08-24T09:00:00Z", "2026-08-24T10:00:00Z", None, None)
            .expect("create entry");

        let files = generate_timesheets(
            &conn,
            "week",
            "2026-08-24",
            out_dir.to_str().unwrap(),
            "Jane Consultant",
            false,
            None,
            Some(contract_id),
            "xlsx",
        )
        .expect("generate timesheets");

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].contract_name, "Only Contract");
        assert!(
            Path::new(&files[0].path).file_name().unwrap().to_string_lossy().contains("Only_Contract"),
            "single-contract filename should include the contract name: {}",
            files[0].path
        );

        let mut workbook = open_workbook_auto(&files[0].path).expect("open generated workbook");
        let sheet_name = workbook.sheet_names()[0].clone();
        let range = workbook.worksheet_range(&sheet_name).expect("read sheet");
        let mut rows = range.rows();
        let header: Vec<String> = rows.next().expect("header row").iter().map(cell_str).collect();
        let contract_col = header.iter().position(|h| h == "Contract");
        assert!(contract_col.is_some(), "every timesheet, single-contract or not, should have a Contract column: {header:?}");

        let data_row = rows.next().expect("one data row");
        assert_eq!(cell_str(&data_row[contract_col.unwrap()]), "Only Contract");

        let _ = fs::remove_file(&db_path);
        let _ = fs::remove_dir_all(&out_dir);
    }

    #[test]
    fn unscoped_timesheet_still_combines_a_clients_contracts_into_one_file() {
        // Regression test for a reported bug: generating timesheets from the
        // top-level Reports view (no client_id/contract_id at all — "every client")
        // was still producing one file per CONTRACT, so a client with two contracts
        // ended up with two separate files instead of one combined one.
        use crate::domain::contracts::{create_client, create_contract};
        use crate::domain::time_entries::create_manual_entry;
        use std::fs;

        let pid = std::process::id();
        let db_path = std::env::temp_dir().join(format!("timetracker_test_timesheets_{pid}_unscoped.sqlite"));
        let _ = fs::remove_file(&db_path);
        let conn = crate::db::open(&db_path).expect("open db");
        let out_dir = std::env::temp_dir().join(format!("timetracker_test_timesheets_{pid}_unscoped_out"));
        let _ = fs::remove_dir_all(&out_dir);
        fs::create_dir_all(&out_dir).expect("create output dir");

        let client_id = create_client(&conn, "Shipping Fast", None).expect("create client");
        let contract_a = create_contract(&conn, client_id, "Contract A", "USD", 50.0).expect("create contract a");
        let contract_b = create_contract(&conn, client_id, "Contract B", "USD", 75.0).expect("create contract b");
        create_manual_entry(&conn, contract_a, "2026-08-24T09:00:00Z", "2026-08-24T10:00:00Z", None, None)
            .expect("create entry a");
        create_manual_entry(&conn, contract_b, "2026-08-25T13:00:00Z", "2026-08-25T14:00:00Z", None, None)
            .expect("create entry b");

        let files = generate_timesheets(
            &conn,
            "week",
            "2026-08-24",
            out_dir.to_str().unwrap(),
            "Jane Consultant",
            false,
            None,
            None,
            "xlsx",
        )
        .expect("generate timesheets");

        assert_eq!(files.len(), 1, "one combined file for the client, not one per contract: {files:?}");
        assert_eq!(files[0].entry_count, 2);
        assert_eq!(files[0].contract_name, "All Contracts");

        let _ = fs::remove_file(&db_path);
        let _ = fs::remove_dir_all(&out_dir);
    }

    #[test]
    fn csv_output_type_writes_a_csv_file_with_a_total_row() {
        use crate::domain::contracts::{create_client, create_contract};
        use crate::domain::time_entries::create_manual_entry;
        use std::fs;

        let pid = std::process::id();
        let db_path = std::env::temp_dir().join(format!("timetracker_test_timesheets_{pid}_csv.sqlite"));
        let _ = fs::remove_file(&db_path);
        let conn = crate::db::open(&db_path).expect("open db");
        let out_dir = std::env::temp_dir().join(format!("timetracker_test_timesheets_{pid}_csv_out"));
        let _ = fs::remove_dir_all(&out_dir);
        fs::create_dir_all(&out_dir).expect("create output dir");

        let client_id = create_client(&conn, "CSV Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "CSV Contract", "USD", 50.0).expect("create contract");
        create_manual_entry(&conn, contract_id, "2026-08-24T09:00:00Z", "2026-08-24T10:30:00Z", None, None)
            .expect("create entry");

        let files = generate_timesheets(
            &conn,
            "week",
            "2026-08-24",
            out_dir.to_str().unwrap(),
            "Jane Consultant",
            true,
            None,
            Some(contract_id),
            "csv",
        )
        .expect("generate timesheets");

        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with(".csv"), "output_type 'csv' should produce a .csv file: {}", files[0].path);

        let mut reader = csv::Reader::from_path(&files[0].path).expect("open generated csv");
        let header = reader.headers().expect("header row").clone();
        assert!(header.iter().any(|h| h == "Contract"), "csv timesheet should have a Contract column: {header:?}");
        assert!(header.iter().any(|h| h == "Rate"), "include_rate_amount should add a Rate column: {header:?}");

        let records: Vec<csv::StringRecord> = reader.records().collect::<Result<Vec<_>, _>>().expect("read rows");
        assert_eq!(records.len(), 2, "one data row plus one total row: {records:?}");
        let end_col = header.iter().position(|h| h == "End Time").unwrap();
        assert_eq!(&records[1][end_col], "Total", "second row should be the totals row");

        let _ = fs::remove_file(&db_path);
        let _ = fs::remove_dir_all(&out_dir);
    }

    #[test]
    fn year_period_covers_the_whole_calendar_year() {
        use crate::domain::contracts::{create_client, create_contract};
        use crate::domain::time_entries::create_manual_entry;
        use std::fs;

        let pid = std::process::id();
        let db_path = std::env::temp_dir().join(format!("timetracker_test_timesheets_{pid}_year.sqlite"));
        let _ = fs::remove_file(&db_path);
        let conn = crate::db::open(&db_path).expect("open db");
        let out_dir = std::env::temp_dir().join(format!("timetracker_test_timesheets_{pid}_year_out"));
        let _ = fs::remove_dir_all(&out_dir);
        fs::create_dir_all(&out_dir).expect("create output dir");

        let client_id = create_client(&conn, "Year Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Year Contract", "USD", 50.0).expect("create contract");
        // One entry in January, one in December — both should land in the same "year" file.
        create_manual_entry(&conn, contract_id, "2026-01-05T09:00:00Z", "2026-01-05T10:00:00Z", None, None)
            .expect("create january entry");
        create_manual_entry(&conn, contract_id, "2026-12-20T09:00:00Z", "2026-12-20T10:00:00Z", None, None)
            .expect("create december entry");

        let files = generate_timesheets(
            &conn,
            "year",
            "2026-06-15",
            out_dir.to_str().unwrap(),
            "Jane Consultant",
            false,
            None,
            Some(contract_id),
            "xlsx",
        )
        .expect("generate timesheets");

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].entry_count, 2, "both January and December entries should fall within the year period");

        let _ = fs::remove_file(&db_path);
        let _ = fs::remove_dir_all(&out_dir);
    }
}
