use std::path::Path;

use chrono::Local;
use rusqlite::types::Value;
use rusqlite::Connection;
use serde::Serialize;

use super::import::read_csv_rows;
use super::purge::{unique_path, write_csv};
use super::DomainResult;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupFile {
    pub data_type: String,
    pub path: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PurgeAllResult {
    pub backups: Vec<BackupFile>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreTypeCount {
    pub data_type: String,
    pub inserted: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    pub restored: Vec<RestoreTypeCount>,
    pub errors: Vec<String>,
}

struct TableSpec {
    data_type: &'static str,
    table_name: &'static str,
    filename_suffix: &'static str,
    sql: &'static str,
    headers: &'static [&'static str],
    /// Header names whose empty CSV value should restore as SQL NULL rather than the
    /// literal empty string (e.g. a nullable `duration_secs` column, which must stay a
    /// real NULL — an empty-string value in that INTEGER column would break every
    /// later read of the row).
    nullable: &'static [&'static str],
    /// Header rows from backups written by older versions, still accepted on restore.
    /// A backup file is identified purely by its header row, so adding a column to
    /// `headers` would otherwise make every previously written file unrecognisable.
    /// Columns a legacy file lacks restore as NULL (see `restore_rows`).
    legacy_headers: &'static [&'static [&'static str]],
}

/// Order matters: earlier tables are restored first so foreign keys they're referenced
/// by (contracts -> clients, contract_rates/time_entries -> contracts, time_entries ->
/// tracking_codes) already exist by the time the dependent rows are inserted.
const TABLES: &[TableSpec] = &[
    TableSpec {
        data_type: "Clients",
        table_name: "clients",
        filename_suffix: "Clients_Backup",
        sql: "SELECT id, name, notes, prime, sub, external_id, week_start, week_end, \
              default_tracking_code_id, archived_at, created_at FROM clients ORDER BY id",
        headers: &[
            "id",
            "name",
            "notes",
            "prime",
            "sub",
            "external_id",
            "week_start",
            "week_end",
            "default_tracking_code_id",
            "archived_at",
            "created_at",
        ],
        nullable: &["notes", "prime", "sub", "external_id", "default_tracking_code_id", "archived_at"],
        legacy_headers: &[],
    },
    TableSpec {
        data_type: "Contracts",
        table_name: "contracts",
        filename_suffix: "Contracts_Backup",
        sql: "SELECT id, client_id, name, currency, external_id, start_date, archived_at, created_at \
              FROM contracts ORDER BY id",
        headers: &["id", "client_id", "name", "currency", "external_id", "start_date", "archived_at", "created_at"],
        nullable: &["external_id", "start_date", "archived_at"],
        legacy_headers: &[],
    },
    TableSpec {
        data_type: "ContractRates",
        table_name: "contract_rates",
        filename_suffix: "ContractRates_Backup",
        sql: "SELECT id, contract_id, hourly_rate, effective_from, created_at FROM contract_rates ORDER BY id",
        headers: &["id", "contract_id", "hourly_rate", "effective_from", "created_at"],
        nullable: &[],
        legacy_headers: &[],
    },
    TableSpec {
        data_type: "Categories",
        table_name: "tracking_codes",
        filename_suffix: "Categories_Backup",
        sql: "SELECT id, client_id, code, description, archived_at, created_at FROM tracking_codes ORDER BY id",
        headers: &["id", "client_id", "code", "description", "archived_at", "created_at"],
        nullable: &["description", "archived_at"],
        legacy_headers: &[],
    },
    TableSpec {
        data_type: "TimeEntries",
        table_name: "time_entries",
        filename_suffix: "TimeEntries_Backup",
        sql: "SELECT id, contract_id, started_at, ended_at, duration_secs, rate_snapshot, notes, source, \
              external_ref, tracking_code_id, created_at, updated_at, deleted_at, local_date, utc_offset_minutes \
              FROM time_entries ORDER BY id",
        headers: &[
            "id",
            "contract_id",
            "started_at",
            "ended_at",
            "duration_secs",
            "rate_snapshot",
            "notes",
            "source",
            "external_ref",
            "tracking_code_id",
            "created_at",
            "updated_at",
            "deleted_at",
            "local_date",
            "utc_offset_minutes",
        ],
        nullable: &[
            "ended_at",
            "duration_secs",
            "notes",
            "external_ref",
            "tracking_code_id",
            "deleted_at",
            "local_date",
            "utc_offset_minutes",
        ],
        // Backups written before entries recorded the day they belong to. Restoring one
        // leaves both columns NULL, which backfill_local_dates then fills in.
        legacy_headers: &[&[
            "id",
            "contract_id",
            "started_at",
            "ended_at",
            "duration_secs",
            "rate_snapshot",
            "notes",
            "source",
            "external_ref",
            "tracking_code_id",
            "created_at",
            "updated_at",
            "deleted_at",
        ]],
    },
];

fn value_to_string(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::Integer(i) => i.to_string(),
        Value::Real(f) => f.to_string(),
        Value::Text(s) => s.clone(),
        Value::Blob(_) => String::new(),
    }
}

fn export_query(conn: &Connection, sql: &str) -> Result<Vec<Vec<String>>, String> {
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let col_count = stmt.column_count();
    let rows = stmt
        .query_map([], |row| {
            (0..col_count)
                .map(|i| row.get::<_, Value>(i).map(|v| value_to_string(&v)))
                .collect::<rusqlite::Result<Vec<String>>>()
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Backs up every table to its own CSV file (one file per data type) in the given
/// folder. Intended for disaster recovery / moving to a new machine — see
/// `restore_from_backups` for the matching import.
pub fn backup_all_data(conn: &Connection, output_folder: &str) -> DomainResult<Vec<BackupFile>> {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let mut results = Vec::new();
    for table in TABLES {
        let rows = export_query(conn, table.sql)?;
        let path = unique_path(output_folder, &format!("{today}_{}.csv", table.filename_suffix));
        write_csv(&path, table.headers, &rows)?;
        results.push(BackupFile {
            data_type: table.data_type.to_string(),
            path: path.to_string_lossy().to_string(),
            count: rows.len() as i64,
        });
    }
    Ok(results)
}

/// `file_header` is the header row as it appeared in the CSV, which may be a legacy
/// layout missing columns this version writes. Values are looked up by column name
/// rather than by position, so a column the file does not carry restores as NULL
/// instead of silently taking whatever sat at that index.
fn restore_rows(
    conn: &Connection,
    table: &TableSpec,
    file_header: &[String],
    rows: &[Vec<String>],
) -> Result<i64, String> {
    let placeholders = (1..=table.headers.len()).map(|i| format!("?{i}")).collect::<Vec<_>>().join(",");
    let sql = format!(
        "INSERT OR IGNORE INTO {} ({}) VALUES ({})",
        table.table_name,
        table.headers.join(","),
        placeholders
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    let mut count = 0i64;
    for row in rows {
        let values: Vec<Option<String>> = table
            .headers
            .iter()
            .map(|header| {
                let raw = file_header
                    .iter()
                    .position(|h| h == header)
                    .and_then(|i| row.get(i))
                    .map(|s| s.as_str())
                    .unwrap_or("");
                if raw.is_empty() && table.nullable.contains(header) {
                    None
                } else {
                    Some(raw.to_string())
                }
            })
            .collect();
        let param_refs: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
        let changed = stmt.execute(param_refs.as_slice()).map_err(|e| e.to_string())?;
        count += changed as i64;
    }
    Ok(count)
}

/// Fills in the day an entry belongs to for any row that arrived without one — rows
/// restored from a backup written before entries recorded it. Identical to what
/// migration 0019 does to pre-existing rows, and for the same reason: an entry with a
/// NULL `local_date` matches no period query at all, so it would be restored into the
/// database and still be invisible on every report.
///
/// Like that migration this reconstructs from this machine's timezone, which is a guess
/// for entries originally logged elsewhere. Backups written by this version carry the
/// recorded values, so nothing is guessed for them.
fn backfill_local_dates(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "UPDATE time_entries
         SET local_date = COALESCE(date(started_at, 'localtime'), substr(started_at, 1, 10)),
             utc_offset_minutes = COALESCE(
                 CAST(round((julianday(datetime(started_at, 'localtime')) - julianday(started_at)) * 1440) AS INTEGER),
                 0
             )
         WHERE local_date IS NULL OR utc_offset_minutes IS NULL",
        [],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Restores from one or more CSV files previously produced by `backup_all_data` (each
/// file's own header row identifies its data type — filenames don't need to match).
/// Rows are inserted with their original ids preserved; a row whose id already exists
/// is left alone rather than overwritten (`INSERT OR IGNORE`), so this is safe to run
/// against a database that already has some data, but it's a pure restore/merge, not a
/// point-in-time rollback — it will never remove or modify existing rows.
pub fn restore_from_backups(conn: &Connection, file_paths: &[String]) -> DomainResult<RestoreResult> {
    let mut matched: Vec<(usize, Vec<String>, Vec<Vec<String>>)> = Vec::new();
    let mut errors = Vec::new();

    for path_str in file_paths {
        let path = Path::new(path_str);
        let label = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| path_str.clone());

        let rows = match read_csv_rows(path) {
            Ok(r) => r,
            Err(e) => {
                errors.push(format!("{label}: {e}"));
                continue;
            }
        };
        if rows.is_empty() {
            errors.push(format!("{label}: file is empty"));
            continue;
        }

        let header = &rows[0];
        let known = TABLES.iter().position(|t| {
            t.headers == header.as_slice() || t.legacy_headers.iter().any(|legacy| *legacy == header.as_slice())
        });
        match known {
            Some(idx) => matched.push((idx, header.clone(), rows[1..].to_vec())),
            None => errors.push(format!("{label}: header doesn't match any known backup type")),
        }
    }

    let mut restored = Vec::new();
    for (idx, table) in TABLES.iter().enumerate() {
        if let Some((_, file_header, rows)) = matched.iter().find(|(i, _, _)| *i == idx) {
            let inserted = restore_rows(conn, table, file_header, rows)?;
            restored.push(RestoreTypeCount { data_type: table.data_type.to_string(), inserted });
        }
    }
    backfill_local_dates(conn)?;

    Ok(RestoreResult { restored, errors })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::domain::contracts::{create_client, create_contract, list_clients, list_contracts};
    use crate::domain::time_entries::{create_manual_entry, list_entries, EntryFilter};
    use crate::domain::tracking_codes::create_tracking_code;
    use std::fs;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("timetracker_test_backup_{}_{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    /// A backup written before entries recorded the day they belong to must still
    /// restore — the file is identified by its header row, so a new column would
    /// otherwise turn every existing backup into "doesn't match any known type" — and
    /// the restored rows must come back visible to period queries rather than sitting
    /// in the database with a NULL day that matches nothing.
    #[test]
    fn a_backup_from_before_local_dates_still_restores_and_stays_queryable() {
        let dir = temp_dir("legacy_restore");
        let db_path = dir.join("db.sqlite");
        let conn = crate::db::open(&db_path).expect("open db");
        let client_id = create_client(&conn, "Legacy Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Legacy", "USD", 50.0).expect("create contract");

        // A CSV in the pre-local_date layout, carrying an instant whose UTC day and
        // local day differ west of Greenwich.
        let legacy_header = TABLES
            .iter()
            .find(|t| t.table_name == "time_entries")
            .expect("time_entries spec")
            .legacy_headers[0];
        let csv_path = dir.join("2026-08-24_TimeEntries_Backup.csv");
        let row = vec![
            "4242".to_string(),
            contract_id.to_string(),
            "2026-08-24T00:30:00Z".to_string(),
            "2026-08-24T01:30:00Z".to_string(),
            "3600".to_string(),
            "50.0".to_string(),
            String::new(),
            "manual".to_string(),
            String::new(),
            String::new(),
            "2026-08-24 01:30:00".to_string(),
            "2026-08-24 01:30:00".to_string(),
            String::new(),
        ];
        write_csv(&csv_path, legacy_header, &[row]).expect("write legacy csv");

        let result = restore_from_backups(&conn, &[csv_path.to_string_lossy().to_string()]).expect("restore");

        assert!(result.errors.is_empty(), "legacy header should be recognised: {:?}", result.errors);
        assert_eq!(result.restored.iter().map(|r| r.inserted).sum::<i64>(), 1);

        // The backfill ran, and the restored entry is reachable by a day-bounded query.
        let (local_date, offset): (String, i32) = conn
            .query_row("SELECT local_date, utc_offset_minutes FROM time_entries WHERE id = 4242", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .expect("restored row");
        let expected = chrono::DateTime::parse_from_rfc3339("2026-08-24T00:30:00Z")
            .unwrap()
            .with_timezone(&Local);
        assert_eq!(local_date, expected.date_naive().format("%Y-%m-%d").to_string());
        assert_eq!(offset, chrono::Offset::fix(expected.offset()).local_minus_utc() / 60);

        let found = list_entries(
            &conn,
            &EntryFilter { contract_id: Some(contract_id), from: Some(local_date.clone()), to: Some(local_date) },
        )
        .expect("list entries");
        assert!(found.iter().any(|e| e.id == 4242), "restored entry must match a day-bounded query");
    }

    /// Backups this version writes carry the recorded day, and restoring one must use
    /// those values rather than reconstructing them from the local timezone.
    #[test]
    fn a_current_backup_round_trips_the_recorded_day() {
        let source_dir = temp_dir("roundtrip_day_src");
        let source = crate::db::open(&source_dir.join("db.sqlite")).expect("open source db");
        let client_id = create_client(&source, "Client", None).expect("create client");
        let contract_id = create_contract(&source, client_id, "Contract", "USD", 50.0).expect("create contract");
        let entry_id = create_manual_entry(&source, contract_id, "2026-08-24T00:30:00Z", "2026-08-24T01:30:00Z", None, None)
            .expect("create entry");
        let original: (String, i32) = source
            .query_row("SELECT local_date, utc_offset_minutes FROM time_entries WHERE id = ?1", [entry_id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .expect("original day");

        let out_dir = temp_dir("roundtrip_day_out");
        let files = backup_all_data(&source, out_dir.to_str().unwrap()).expect("backup");
        let paths: Vec<String> = files.iter().map(|f| f.path.clone()).collect();

        let dest_dir = temp_dir("roundtrip_day_dest");
        let dest = crate::db::open(&dest_dir.join("db.sqlite")).expect("open dest db");
        restore_from_backups(&dest, &paths).expect("restore");

        let restored: (String, i32) = dest
            .query_row("SELECT local_date, utc_offset_minutes FROM time_entries WHERE id = ?1", [entry_id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .expect("restored day");
        assert_eq!(restored, original);
    }

    #[test]
    fn backup_then_restore_round_trips_all_data_into_a_fresh_database() {
        let source_dir = temp_dir("source");
        let source = crate::db::open(&source_dir.join("db.sqlite")).expect("open source db");

        let client_id = create_client(&source, "Round Trip Client", None).expect("create client");
        let contract_id =
            create_contract(&source, client_id, "Round Trip Contract", "USD", 75.0).expect("create contract");
        let category_id = create_tracking_code(&source, client_id, "BILLABLE", Some("desc")).expect("create category");
        let entry_id = create_manual_entry(
            &source,
            contract_id,
            "2026-08-01T09:00:00Z",
            "2026-08-01T11:00:00Z",
            Some("round trip entry"),
            Some(category_id),
        )
        .expect("create entry");

        let files = backup_all_data(&source, source_dir.to_str().unwrap()).expect("backup");
        assert_eq!(files.len(), 5);
        for f in &files {
            assert!(Path::new(&f.path).exists(), "{} backup file should exist", f.data_type);
        }

        let dest_dir = temp_dir("dest");
        let dest = crate::db::open(&dest_dir.join("db.sqlite")).expect("open fresh dest db");
        let file_paths: Vec<String> = files.iter().map(|f| f.path.clone()).collect();
        let restore_result = restore_from_backups(&dest, &file_paths).expect("restore");
        assert!(restore_result.errors.is_empty(), "errors: {:?}", restore_result.errors);

        let clients = list_clients(&dest, true).expect("list clients");
        assert_eq!(clients.len(), 1);
        assert_eq!(clients[0].id, client_id);
        assert_eq!(clients[0].name, "Round Trip Client");

        let contracts = list_contracts(&dest, true).expect("list contracts");
        assert_eq!(contracts.len(), 1);
        assert_eq!(contracts[0].id, contract_id);
        assert_eq!(contracts[0].current_rate, Some(75.0), "rate history restored correctly");

        let entries = list_entries(&dest, &EntryFilter::default()).expect("list entries");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, entry_id);
        assert_eq!(entries[0].notes.as_deref(), Some("round trip entry"));
        assert_eq!(entries[0].tracking_code.as_deref(), Some("BILLABLE"));
    }

    #[test]
    fn restore_does_not_duplicate_or_overwrite_existing_rows() {
        let dir = temp_dir("no_dup");
        let conn = crate::db::open(&dir.join("db.sqlite")).expect("open db");

        let client_id = create_client(&conn, "Existing Client", None).expect("create client");
        create_contract(&conn, client_id, "Existing Contract", "USD", 10.0).expect("create contract");

        let files = backup_all_data(&conn, dir.to_str().unwrap()).expect("backup");
        let file_paths: Vec<String> = files.iter().map(|f| f.path.clone()).collect();

        // Restoring the exact same backup into the same (non-empty) database should be
        // a no-op — every row's id already exists, so nothing new is inserted.
        let result = restore_from_backups(&conn, &file_paths).expect("restore into same db");
        for r in &result.restored {
            assert_eq!(r.inserted, 0, "{} should have inserted nothing new: {:?}", r.data_type, result.restored);
        }

        let clients = list_clients(&conn, true).expect("list clients");
        assert_eq!(clients.len(), 1, "should still be exactly one client, not duplicated");
    }
}
