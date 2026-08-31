use std::path::Path;

use calamine::{open_workbook_auto, Data, Reader};
use chrono::{Local, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use rusqlite::{params, Connection};
use serde::Serialize;

use super::import_templates::get_import_template;
use super::time_entries::create_manual_entry;
use super::tracking_codes::{create_tracking_code, list_tracking_codes, TrackingCode};
use super::DomainResult;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub imported: i64,
    pub skipped: i64,
    pub errors: Vec<String>,
}

fn client_id_for_contract(conn: &Connection, contract_id: i64) -> DomainResult<i64> {
    conn.query_row(
        "SELECT client_id FROM contracts WHERE id = ?1",
        params![contract_id],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

pub(crate) fn read_csv_rows(path: &Path) -> Result<Vec<Vec<String>>, String> {
    let mut reader = csv::Reader::from_path(path).map_err(|e| e.to_string())?;
    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| e.to_string())?
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut rows = vec![headers];
    for result in reader.records() {
        let record = result.map_err(|e| e.to_string())?;
        rows.push(record.iter().map(|s| s.to_string()).collect());
    }
    Ok(rows)
}

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) => {
            if f.fract() == 0.0 {
                format!("{}", *f as i64)
            } else {
                f.to_string()
            }
        }
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(dt) => dt
            .as_datetime()
            .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default(),
        Data::DateTimeIso(s) => s.clone(),
        Data::DurationIso(s) => s.clone(),
        Data::Error(e) => format!("#ERROR:{e:?}"),
    }
}

/// A block of rows to process as one unit — either a whole CSV file, or a single
/// worksheet tab within an XLSX workbook (a workbook with multiple tabs produces one
/// `RowSource` per tab, each with its own header row).
struct RowSource {
    label: String,
    rows: Vec<Vec<String>>,
}

fn read_xlsx_sources(path: &Path, file_label: &str) -> Result<Vec<RowSource>, String> {
    let mut workbook = open_workbook_auto(path).map_err(|e| e.to_string())?;
    let sheet_names = workbook.sheet_names().to_vec();
    if sheet_names.is_empty() {
        return Err("workbook has no sheets".to_string());
    }
    let multiple_sheets = sheet_names.len() > 1;

    let mut sources = Vec::new();
    for sheet_name in sheet_names {
        let range = workbook.worksheet_range(&sheet_name).map_err(|e| e.to_string())?;
        let rows: Vec<Vec<String>> = range.rows().map(|row| row.iter().map(cell_to_string).collect()).collect();
        let label = if multiple_sheets {
            format!("{file_label} [{sheet_name}]")
        } else {
            file_label.to_string()
        };
        sources.push(RowSource { label, rows });
    }
    Ok(sources)
}

fn find_col(headers: &[String], name: &str) -> Option<usize> {
    headers.iter().position(|h| h.trim().eq_ignore_ascii_case(name))
}

/// Accepts the app's own ISO export format as well as common spreadsheet date formats
/// (e.g. "8/24/2026" from US-style timesheets), including non-zero-padded month/day.
/// Excel date cells can come through as a full "YYYY-MM-DD HH:MM:SS" string (the time
/// is always midnight for a genuine date-only cell) — the leading date part is used.
fn parse_date(s: &str) -> Result<NaiveDate, String> {
    let full = s.trim();
    let s = full.split_whitespace().next().unwrap_or(full);
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%m/%d/%Y") {
        return Ok(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%m-%d-%Y") {
        return Ok(d);
    }
    // Non-zero-padded "M/D/YYYY" (e.g. "8/24/2026") — chrono's %m/%d expect exact width.
    let parts: Vec<&str> = s.split('/').collect();
    if let [m, d, y] = parts[..] {
        if let (Ok(m), Ok(d), Ok(y)) = (m.parse::<u32>(), d.parse::<u32>(), y.parse::<i32>()) {
            if let Some(date) = NaiveDate::from_ymd_opt(y, m, d) {
                return Ok(date);
            }
        }
    }
    Err(format!("invalid date '{full}'"))
}

/// Excel time-of-day cells can come through as a full "YYYY-MM-DD HH:MM:SS" string
/// (with a placeholder date, typically the 1899/1900 epoch) — the trailing time part is
/// used in that case. Accepts both 24-hour ("14:30") and 12-hour with AM/PM
/// ("2:30 PM", "2:30PM", "2:30:00 pm") — whichever the source spreadsheet uses; the
/// importer doesn't need to be told which in advance.
fn parse_time(s: &str) -> Result<NaiveTime, String> {
    let full = s.trim();
    // Only strip a "date " prefix when it actually looks like one (contains '-', as
    // in Excel's placeholder-date format) — a plain "2:30 PM" has a space too, but
    // splitting on it unconditionally would throw away the "2:30" and leave just "PM".
    let s = match full.split_once(' ') {
        Some((prefix, time)) if prefix.contains('-') => time,
        _ => full,
    };
    if let Ok(t) = NaiveTime::parse_from_str(s, "%H:%M") {
        return Ok(t);
    }
    if let Ok(t) = NaiveTime::parse_from_str(s, "%H:%M:%S") {
        return Ok(t);
    }
    // 12-hour AM/PM formats. chrono's %p expects "AM"/"PM" — uppercase first so
    // "am"/"pm"/"Am" etc. from real-world spreadsheets still match. Also accept no
    // space before the meridiem ("2:30PM") by inserting one if it's missing.
    let upper = s.to_uppercase();
    let spaced = if upper.ends_with("AM") || upper.ends_with("PM") {
        let (time_part, meridiem) = upper.split_at(upper.len() - 2);
        if time_part.ends_with(' ') {
            upper.clone()
        } else {
            format!("{time_part} {meridiem}")
        }
    } else {
        upper.clone()
    };
    if let Ok(t) = NaiveTime::parse_from_str(&spaced, "%I:%M %p") {
        return Ok(t);
    }
    if let Ok(t) = NaiveTime::parse_from_str(&spaced, "%I:%M:%S %p") {
        return Ok(t);
    }
    Err(format!("invalid time '{full}'"))
}

fn local_naive_to_utc_rfc3339(naive: NaiveDateTime) -> Result<String, String> {
    match Local.from_local_datetime(&naive) {
        LocalResult::Single(dt) | LocalResult::Ambiguous(dt, _) => Ok(dt.with_timezone(&chrono::Utc).to_rfc3339()),
        LocalResult::None => Err("that local date/time does not exist (DST gap)".to_string()),
    }
}

/// Parses a Date + Start Time + End Time (local time, matching how the app displays
/// times elsewhere) into UTC RFC3339 bounds. If the end time is not after the start
/// time on the same date, the entry is assumed to run past midnight and the end date
/// rolls forward one day, rather than being rejected outright.
fn parse_row_times(date_str: &str, start_str: &str, end_str: &str) -> Result<(String, String), String> {
    let date = parse_date(date_str)?;
    let start_time = parse_time(start_str)?;
    let end_time = parse_time(end_str)?;

    let end_date = if end_time <= start_time {
        date.succ_opt().ok_or_else(|| "date out of range".to_string())?
    } else {
        date
    };

    let started_at = local_naive_to_utc_rfc3339(date.and_time(start_time))?;
    let ended_at = local_naive_to_utc_rfc3339(end_date.and_time(end_time))?;
    Ok((started_at, ended_at))
}

/// Imports time entries from one or more CSV/XLSX files against a single contract,
/// using `template_id` to know which column names to look for (see
/// `import_templates`). An XLSX workbook with multiple tabs has every tab imported
/// (each tab is treated as its own block of rows, with its own header row). Column
/// matching is case-insensitive; extra columns (like a computed "Hours") are ignored
/// since duration is derived from Start/End directly. The template's category/notes
/// columns are optional — leave one blank in the template to skip looking for it
/// entirely. Rows with a date but blank Start/End are skipped silently (e.g. a
/// placeholder zero-hour day), rather than reported as errors. Times accept both
/// 24-hour and 12-hour-with-AM/PM formats.
pub fn import_time_entries(
    conn: &Connection,
    contract_id: i64,
    file_paths: &[String],
    template_id: i64,
) -> DomainResult<ImportResult> {
    let template = get_import_template(conn, template_id)?;
    let client_id = client_id_for_contract(conn, contract_id)?;
    let mut categories = list_tracking_codes(conn, client_id, false)?;

    let mut imported = 0i64;
    let mut skipped = 0i64;
    let mut errors = Vec::new();

    for path_str in file_paths {
        let path = Path::new(path_str);
        let file_label = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path_str.clone());
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

        let sources = match ext.as_str() {
            "csv" => read_csv_rows(path).map(|rows| vec![RowSource { label: file_label.clone(), rows }]),
            "xlsx" | "xls" | "xlsm" => read_xlsx_sources(path, &file_label),
            other => Err(format!("unsupported file type '.{other}'")),
        };

        let sources = match sources {
            Ok(s) => s,
            Err(e) => {
                errors.push(format!("{file_label}: {e}"));
                continue;
            }
        };

        for source in sources {
            let label = source.label;
            let rows = source.rows;

            if rows.is_empty() {
                errors.push(format!("{label}: sheet is empty"));
                continue;
            }

            let headers = &rows[0];
            let date_col = find_col(headers, &template.date_column);
            let start_col = find_col(headers, &template.start_column);
            let end_col = find_col(headers, &template.end_column);
            let category_col = template.category_column.as_deref().and_then(|name| find_col(headers, name));
            let notes_col = template.notes_column.as_deref().and_then(|name| find_col(headers, name));

            let (date_col, start_col, end_col) = match (date_col, start_col, end_col) {
                (Some(d), Some(s), Some(e)) => (d, s, e),
                _ => {
                    errors.push(format!(
                        "{label}: missing required column(s) — template '{}' expects '{}', '{}', '{}'",
                        template.name, template.date_column, template.start_column, template.end_column
                    ));
                    continue;
                }
            };

            for (row_idx, row) in rows.iter().enumerate().skip(1) {
                let get = |col: usize| row.get(col).map(|s| s.as_str()).unwrap_or("");
                let date_str = get(date_col);
                if date_str.trim().is_empty() {
                    continue;
                }
                if get(start_col).trim().is_empty() && get(end_col).trim().is_empty() {
                    continue;
                }

                let notes = notes_col
                    .and_then(|c| row.get(c))
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty());
                let category_text = category_col
                    .and_then(|c| row.get(c))
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty());

                let result: Result<(), String> = (|| {
                    let (started_at, ended_at) = parse_row_times(date_str, get(start_col), get(end_col))?;

                    // A category named in the import that doesn't already exist for this
                    // client is added to the client's categories on the fly, rather than
                    // being rejected — bulk-imported historical data is often the first
                    // time a client's categories get written down at all.
                    let tracking_code_id = match category_text {
                        Some(text) => {
                            let existing = categories.iter().find(|c| c.code.eq_ignore_ascii_case(text)).map(|c| c.id);
                            let id = match existing {
                                Some(id) => id,
                                None => {
                                    let new_id = create_tracking_code(conn, client_id, text, None)?;
                                    categories.push(TrackingCode {
                                        id: new_id,
                                        client_id,
                                        code: text.to_string(),
                                        description: None,
                                        archived_at: None,
                                    });
                                    new_id
                                }
                            };
                            Some(id)
                        }
                        None => None,
                    };

                    create_manual_entry(conn, contract_id, &started_at, &ended_at, notes, tracking_code_id)?;
                    Ok(())
                })();

                match result {
                    Ok(()) => imported += 1,
                    Err(e) => {
                        errors.push(format!("{label} row {}: {e}", row_idx + 1));
                        skipped += 1;
                    }
                }
            }
        }
    }

    Ok(ImportResult { imported, skipped, errors })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::contracts::{create_client, create_contract};
    use crate::domain::time_entries::{list_entries, EntryFilter};
    use rust_xlsxwriter::Workbook;
    use std::fs;

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("timetracker_test_{}_{name}", std::process::id()))
    }

    /// Both starter templates seeded by migration 0012 are present in every fresh
    /// test database — looked up by name rather than assumed id, in case seed order
    /// ever changes.
    fn template_id_named(conn: &Connection, name: &str) -> i64 {
        super::super::import_templates::list_import_templates(conn)
            .expect("list import templates")
            .into_iter()
            .find(|t| t.name == name)
            .unwrap_or_else(|| panic!("seeded import template '{name}' not found"))
            .id
    }

    #[test]
    fn parses_dates_and_times_with_spurious_time_or_date_suffix() {
        // Excel date-only cells surface as "YYYY-MM-DD HH:MM:SS" with a midnight time
        // component — this is the exact bug reported in production:
        // "invalid date '2026-08-03 00:00:00'".
        assert_eq!(parse_date("2026-08-03 00:00:00").unwrap(), NaiveDate::from_ymd_opt(2026, 8, 3).unwrap());
        assert_eq!(parse_date("2026-08-03").unwrap(), NaiveDate::from_ymd_opt(2026, 8, 3).unwrap());
        assert_eq!(parse_date("8/24/2026").unwrap(), NaiveDate::from_ymd_opt(2026, 8, 24).unwrap());

        // Excel time-of-day cells can likewise surface with a placeholder epoch date.
        assert_eq!(parse_time("1899-12-30 11:30:00").unwrap(), NaiveTime::from_hms_opt(11, 30, 0).unwrap());
        assert_eq!(parse_time("11:30").unwrap(), NaiveTime::from_hms_opt(11, 30, 0).unwrap());
    }

    #[test]
    fn parses_12_hour_times_with_am_pm() {
        assert_eq!(parse_time("2:30 PM").unwrap(), NaiveTime::from_hms_opt(14, 30, 0).unwrap());
        assert_eq!(parse_time("2:30PM").unwrap(), NaiveTime::from_hms_opt(14, 30, 0).unwrap());
        assert_eq!(parse_time("2:30 pm").unwrap(), NaiveTime::from_hms_opt(14, 30, 0).unwrap());
        assert_eq!(parse_time("9:00 AM").unwrap(), NaiveTime::from_hms_opt(9, 0, 0).unwrap());
        assert_eq!(parse_time("12:00 AM").unwrap(), NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        assert_eq!(parse_time("12:00 PM").unwrap(), NaiveTime::from_hms_opt(12, 0, 0).unwrap());
        assert_eq!(parse_time("2:30:15 PM").unwrap(), NaiveTime::from_hms_opt(14, 30, 15).unwrap());
    }

    #[test]
    fn imports_csv_and_xlsx_including_overnight_rollover() {
        let db_path = temp_path("import.sqlite");
        let _ = fs::remove_file(&db_path);
        let conn = crate::db::open(&db_path).expect("open db");

        let client_id = create_client(&conn, "Test Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Test Contract", "USD", 50.0).expect("create contract");
        let template_id = template_id_named(&conn, "Default");

        let csv_path = temp_path("import.csv");
        fs::write(
            &csv_path,
            "Date,Start Time,End Time,Category,Notes\n2026-08-21,09:00,11:30,,Worked on CSV import\n",
        )
        .expect("write csv");

        let csv_result =
            import_time_entries(&conn, contract_id, &[csv_path.to_string_lossy().to_string()], template_id)
                .expect("import csv");
        assert_eq!(csv_result.imported, 1, "errors: {:?}", csv_result.errors);
        assert_eq!(csv_result.skipped, 0);

        let xlsx_path = temp_path("import.xlsx");
        let mut workbook = Workbook::new();
        let sheet = workbook.add_worksheet();
        for (col, header) in ["Date", "Start Time", "End Time", "Category", "Notes"].iter().enumerate() {
            sheet.write(0, col as u16, *header).unwrap();
        }
        sheet.write(1, 0, "2026-08-22").unwrap();
        sheet.write(1, 1, "23:00").unwrap();
        sheet.write(1, 2, "01:00").unwrap();
        sheet.write(1, 3, "").unwrap();
        sheet.write(1, 4, "Overnight shift").unwrap();
        workbook.save(&xlsx_path).expect("save xlsx");

        let xlsx_result =
            import_time_entries(&conn, contract_id, &[xlsx_path.to_string_lossy().to_string()], template_id)
                .expect("import xlsx");
        assert_eq!(xlsx_result.imported, 1, "errors: {:?}", xlsx_result.errors);

        let entries = list_entries(&conn, &EntryFilter::default()).expect("list entries");
        assert_eq!(entries.len(), 2);

        let overnight = entries
            .iter()
            .find(|e| e.notes.as_deref() == Some("Overnight shift"))
            .expect("overnight entry present");
        assert_eq!(overnight.duration_secs, Some(2 * 3600));

        let csv_entry = entries
            .iter()
            .find(|e| e.notes.as_deref() == Some("Worked on CSV import"))
            .expect("csv entry present");
        assert_eq!(csv_entry.duration_secs, Some(2 * 3600 + 1800));

        let _ = fs::remove_file(&db_path);
        let _ = fs::remove_file(&csv_path);
        let _ = fs::remove_file(&xlsx_path);
    }

    #[test]
    fn imports_multi_tab_xlsx_with_activity_column_and_us_dates() {
        let db_path = temp_path("import_tabs.sqlite");
        let _ = fs::remove_file(&db_path);
        let conn = crate::db::open(&db_path).expect("open db");

        let client_id = create_client(&conn, "Tab Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Tab Contract", "USD", 40.0).expect("create contract");
        let template_id = template_id_named(&conn, "Activity Sheet");

        let xlsx_path = temp_path("import_tabs.xlsx");
        let mut workbook = Workbook::new();

        // Tab 1: mirrors the real-world layout — Date, Start, End, Hours (formula-like,
        // ignored by the importer), Category, Activity — including a placeholder row
        // with a date but no start/end, which should be skipped rather than erroring.
        let sheet1 = workbook.add_worksheet().set_name("Week1").unwrap();
        for (col, header) in ["Date", "Start", "End", "Hours", "Category", "Activity"].iter().enumerate() {
            sheet1.write(0, col as u16, *header).unwrap();
        }
        sheet1.write(1, 0, "8/24/2026").unwrap();
        sheet1.write(1, 1, "11:30").unwrap();
        sheet1.write(1, 2, "11:45").unwrap();
        sheet1.write(1, 3, 0.25 / 24.0).unwrap();
        sheet1.write(1, 4, "Software configuration and interface").unwrap();
        sheet1.write(1, 5, "").unwrap();

        sheet1.write(2, 0, "8/25/2026").unwrap();
        sheet1.write(2, 1, "").unwrap();
        sheet1.write(2, 2, "").unwrap();
        sheet1.write(2, 3, 0.0).unwrap();
        sheet1.write(2, 4, "Software configuration and interface").unwrap();
        sheet1.write(2, 5, "").unwrap();

        sheet1.write(3, 0, "8/26/2026").unwrap();
        sheet1.write(3, 1, "11:00").unwrap();
        sheet1.write(3, 2, "12:30").unwrap();
        sheet1.write(3, 3, 1.5 / 24.0).unwrap();
        sheet1.write(3, 4, "Software configuration and interface").unwrap();
        sheet1.write(3, 5, "PSM Certificate issues").unwrap();

        // Tab 2: a second week, proving multi-tab workbooks are fully imported.
        let sheet2 = workbook.add_worksheet().set_name("Week2").unwrap();
        for (col, header) in ["Date", "Start", "End", "Hours", "Category", "Activity"].iter().enumerate() {
            sheet2.write(0, col as u16, *header).unwrap();
        }
        sheet2.write(1, 0, "9/1/2026").unwrap();
        sheet2.write(1, 1, "09:00").unwrap();
        sheet2.write(1, 2, "10:00").unwrap();
        sheet2.write(1, 3, 1.0 / 24.0).unwrap();
        sheet2.write(1, 4, "Software configuration and interface").unwrap();
        sheet2.write(1, 5, "Week 2 work").unwrap();

        workbook.save(&xlsx_path).expect("save xlsx");

        let result =
            import_time_entries(&conn, contract_id, &[xlsx_path.to_string_lossy().to_string()], template_id)
                .expect("import xlsx");
        assert_eq!(result.imported, 3, "errors: {:?}", result.errors);
        assert_eq!(result.skipped, 0, "errors: {:?}", result.errors);

        let entries = list_entries(&conn, &EntryFilter::default()).expect("list entries");
        assert_eq!(entries.len(), 3);

        let cert_entry = entries
            .iter()
            .find(|e| e.notes.as_deref() == Some("PSM Certificate issues"))
            .expect("Aug 26 entry present with correct US-format date parsing");
        assert_eq!(cert_entry.duration_secs, Some(3600 + 1800));

        let week2_entry = entries
            .iter()
            .find(|e| e.notes.as_deref() == Some("Week 2 work"))
            .expect("second tab's entry was imported");
        assert_eq!(week2_entry.duration_secs, Some(3600));

        let _ = fs::remove_file(&db_path);
        let _ = fs::remove_file(&xlsx_path);
    }

    #[test]
    fn auto_creates_new_categories_and_allows_blank_category() {
        let db_path = temp_path("import_cat.sqlite");
        let _ = fs::remove_file(&db_path);
        let conn = crate::db::open(&db_path).expect("open db");

        let client_id = create_client(&conn, "Categorized Client", None).expect("create client");
        let contract_id =
            create_contract(&conn, client_id, "Categorized Contract", "USD", 50.0).expect("create contract");
        super::super::tracking_codes::create_tracking_code(&conn, client_id, "BILLABLE", None)
            .expect("create category");
        let template_id = template_id_named(&conn, "Default");

        let csv_path = temp_path("import_cat.csv");
        fs::write(
            &csv_path,
            "Date,Start Time,End Time,Category,Notes\n\
             2026-08-21,09:00,10:00,BILLABLE,existing category\n\
             2026-08-21,10:00,11:00,NEW CATEGORY,unseen category gets created\n\
             2026-08-21,12:00,13:00,new category,reuses the one just created (case-insensitive)\n\
             2026-08-21,14:00,13:30,,blank category is fine even though this client has categories\n",
        )
        .expect("write csv");

        let result =
            import_time_entries(&conn, contract_id, &[csv_path.to_string_lossy().to_string()], template_id)
                .expect("import csv");
        assert_eq!(result.imported, 4, "errors: {:?}", result.errors);
        assert_eq!(result.skipped, 0, "errors: {:?}", result.errors);

        let categories = list_tracking_codes(&conn, client_id, false).expect("list categories");
        assert_eq!(categories.len(), 2, "should have BILLABLE plus the one newly created category: {categories:?}");
        assert!(categories.iter().any(|c| c.code.eq_ignore_ascii_case("NEW CATEGORY")));

        let entries = list_entries(&conn, &EntryFilter::default()).expect("list entries");
        let new_cat_entries: Vec<_> = entries
            .iter()
            .filter(|e| e.tracking_code.as_deref().map(|c| c.eq_ignore_ascii_case("new category")) == Some(true))
            .collect();
        assert_eq!(new_cat_entries.len(), 2, "both rows should share the single auto-created category");

        let blank_entry = entries
            .iter()
            .find(|e| e.notes.as_deref() == Some("blank category is fine even though this client has categories"))
            .expect("blank-category entry was imported");
        assert_eq!(blank_entry.tracking_code_id, None);

        let _ = fs::remove_file(&db_path);
        let _ = fs::remove_file(&csv_path);
    }
}
