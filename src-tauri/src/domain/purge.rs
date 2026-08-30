use std::path::{Path, PathBuf};

use chrono::Local;
use csv::Writer;
use rusqlite::{params, Connection};
use serde::Serialize;

use super::DomainResult;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PurgeResult {
    pub count: i64,
    pub backup_path: String,
    /// Human-readable reasons any candidate row was skipped (still referenced by
    /// something else) — the CSV/delete only ever covers the rows that weren't blocked.
    pub blocked: Vec<String>,
}

fn today() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// Avoids overwriting a same-day backup by appending `_2`, `_3`, ... if the target
/// filename already exists.
pub(crate) fn unique_path(output_folder: &str, filename: &str) -> PathBuf {
    let base = Path::new(output_folder).join(filename);
    if !base.exists() {
        return base;
    }
    let stem = Path::new(filename)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| filename.to_string());
    let ext = Path::new(filename).extension().and_then(|e| e.to_str()).unwrap_or("csv");
    for i in 2..1000 {
        let candidate = Path::new(output_folder).join(format!("{stem}_{i}.{ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    base
}

pub(crate) fn write_csv(path: &Path, headers: &[&str], rows: &[Vec<String>]) -> Result<(), String> {
    let mut writer = Writer::from_path(path).map_err(|e| e.to_string())?;
    writer.write_record(headers).map_err(|e| e.to_string())?;
    for row in rows {
        writer.write_record(row).map_err(|e| e.to_string())?;
    }
    writer.flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn count(conn: &Connection, sql: &str, id: i64) -> Result<i64, String> {
    conn.query_row(sql, params![id], |row| row.get(0)).map_err(|e| e.to_string())
}

fn delete_by_ids(conn: &Connection, table: &str, ids: &[i64]) -> Result<(), String> {
    if ids.is_empty() {
        return Ok(());
    }
    let id_list = ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
    conn.execute(&format!("DELETE FROM {table} WHERE id IN ({id_list})"), []).map(|_| ()).map_err(|e| e.to_string())
}

/// Permanently deletes soft-deleted time entries whose `deleted_at` is on/after
/// `cutoff` (all of them if `cutoff` is `None`) — matching whatever range the Trash
/// page's date filter currently shows. A CSV backup of exactly what's being removed is
/// always written first, since this is not recoverable afterward. Time entries have no
/// dependents, so nothing here can be blocked.
pub fn purge_deleted_entries(
    conn: &Connection,
    cutoff: Option<&str>,
    output_folder: &str,
) -> DomainResult<PurgeResult> {
    let mut stmt = conn
        .prepare(
            "SELECT te.id, cl.name, c.name, te.started_at, te.ended_at, te.duration_secs, te.rate_snapshot,
                    te.notes, te.source, tc.code, te.deleted_at
             FROM time_entries te
             JOIN contracts c ON c.id = te.contract_id
             JOIN clients cl ON cl.id = c.client_id
             LEFT JOIN tracking_codes tc ON tc.id = te.tracking_code_id
             WHERE te.deleted_at IS NOT NULL AND (?1 IS NULL OR te.deleted_at >= ?1)
             ORDER BY te.deleted_at",
        )
        .map_err(|e| e.to_string())?;

    #[allow(clippy::type_complexity)]
    let query_rows: Vec<(i64, String, String, String, Option<String>, Option<i64>, f64, Option<String>, String, Option<String>, String)> =
        stmt.query_map(params![cutoff], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
                row.get(9)?,
                row.get(10)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut ids = Vec::new();
    let mut rows = Vec::new();
    for (id, client, contract, started_at, ended_at, duration_secs, rate, notes, source, category, deleted_at) in
        query_rows
    {
        ids.push(id);
        rows.push(vec![
            id.to_string(),
            client,
            contract,
            started_at,
            ended_at.unwrap_or_default(),
            duration_secs.map(|d| d.to_string()).unwrap_or_default(),
            rate.to_string(),
            notes.unwrap_or_default(),
            source,
            category.unwrap_or_default(),
            deleted_at,
        ]);
    }

    let path = unique_path(output_folder, &format!("{}_Entries_Deleted_Backup_Purge.csv", today()));
    write_csv(
        &path,
        &[
            "id",
            "client",
            "contract",
            "started_at",
            "ended_at",
            "duration_secs",
            "rate_snapshot",
            "notes",
            "source",
            "category",
            "deleted_at",
        ],
        &rows,
    )?;

    let count = ids.len() as i64;
    delete_by_ids(conn, "time_entries", &ids)?;

    Ok(PurgeResult { count, backup_path: path.to_string_lossy().to_string(), blocked: Vec::new() })
}

pub fn purge_archived_contracts(
    conn: &Connection,
    cutoff: Option<&str>,
    output_folder: &str,
) -> DomainResult<PurgeResult> {
    let mut stmt = conn
        .prepare(
            "SELECT c.id, cl.name, c.name, c.currency, c.external_id, c.start_date, c.archived_at
             FROM contracts c JOIN clients cl ON cl.id = c.client_id
             WHERE c.archived_at IS NOT NULL AND (?1 IS NULL OR c.archived_at >= ?1)
             ORDER BY c.archived_at",
        )
        .map_err(|e| e.to_string())?;

    #[allow(clippy::type_complexity)]
    let query_rows: Vec<(i64, String, String, String, Option<String>, Option<String>, String)> = stmt
        .query_map(params![cutoff], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut ids = Vec::new();
    let mut rows = Vec::new();
    let mut blocked = Vec::new();
    for (id, client, name, currency, external_id, start_date, archived_at) in query_rows {
        let entry_count = count(conn, "SELECT COUNT(*) FROM time_entries WHERE contract_id = ?1", id)?;
        if entry_count > 0 {
            blocked.push(format!(
                "{client} / {name}: {entry_count} time entr{} still reference it (delete those first)",
                if entry_count == 1 { "y" } else { "ies" }
            ));
            continue;
        }
        ids.push(id);
        rows.push(vec![
            id.to_string(),
            client,
            name,
            currency,
            external_id.unwrap_or_default(),
            start_date.unwrap_or_default(),
            archived_at,
        ]);
    }

    let path = unique_path(output_folder, &format!("{}_Contract_Archive_Backup_Purge.csv", today()));
    write_csv(
        &path,
        &["id", "client", "contract", "currency", "external_id", "start_date", "archived_at"],
        &rows,
    )?;

    let count = ids.len() as i64;
    // Rate history has no independent meaning once its contract is gone, so it's
    // cascaded here (already confirmed to have no other dependents above).
    if !ids.is_empty() {
        let id_list = ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
        conn.execute(&format!("DELETE FROM contract_rates WHERE contract_id IN ({id_list})"), [])
            .map_err(|e| e.to_string())?;
    }
    delete_by_ids(conn, "contracts", &ids)?;

    Ok(PurgeResult { count, backup_path: path.to_string_lossy().to_string(), blocked })
}

pub fn purge_archived_clients(
    conn: &Connection,
    cutoff: Option<&str>,
    output_folder: &str,
) -> DomainResult<PurgeResult> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, notes, prime, sub, external_id, week_start, week_end, archived_at
             FROM clients WHERE archived_at IS NOT NULL AND (?1 IS NULL OR archived_at >= ?1)
             ORDER BY archived_at",
        )
        .map_err(|e| e.to_string())?;

    #[allow(clippy::type_complexity)]
    let query_rows: Vec<(i64, String, Option<String>, Option<String>, Option<String>, Option<String>, String, String, String)> =
        stmt.query_map(params![cutoff], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut ids = Vec::new();
    let mut rows = Vec::new();
    let mut blocked = Vec::new();
    for (id, name, notes, prime, sub, external_id, week_start, week_end, archived_at) in query_rows {
        let contract_count = count(conn, "SELECT COUNT(*) FROM contracts WHERE client_id = ?1", id)?;
        let category_count = count(conn, "SELECT COUNT(*) FROM tracking_codes WHERE client_id = ?1", id)?;
        if contract_count > 0 || category_count > 0 {
            let mut reasons = Vec::new();
            if contract_count > 0 {
                reasons.push(format!(
                    "{contract_count} contract{} (including any not archived)",
                    if contract_count == 1 { "" } else { "s" }
                ));
            }
            if category_count > 0 {
                reasons.push(format!(
                    "{category_count} categor{} (including any not archived)",
                    if category_count == 1 { "y" } else { "ies" }
                ));
            }
            blocked.push(format!("{name}: {} still reference it (purge those first)", reasons.join(" and ")));
            continue;
        }
        ids.push(id);
        rows.push(vec![
            id.to_string(),
            name,
            notes.unwrap_or_default(),
            prime.unwrap_or_default(),
            sub.unwrap_or_default(),
            external_id.unwrap_or_default(),
            week_start,
            week_end,
            archived_at,
        ]);
    }

    let path = unique_path(output_folder, &format!("{}_Client_Archive_Backup_Purge.csv", today()));
    write_csv(
        &path,
        &["id", "name", "notes", "prime", "sub", "external_id", "week_start", "week_end", "archived_at"],
        &rows,
    )?;

    let count = ids.len() as i64;
    delete_by_ids(conn, "clients", &ids)?;

    Ok(PurgeResult { count, backup_path: path.to_string_lossy().to_string(), blocked })
}

pub fn purge_archived_categories(
    conn: &Connection,
    cutoff: Option<&str>,
    output_folder: &str,
) -> DomainResult<PurgeResult> {
    let mut stmt = conn
        .prepare(
            "SELECT tc.id, cl.name, tc.code, tc.description, tc.archived_at
             FROM tracking_codes tc JOIN clients cl ON cl.id = tc.client_id
             WHERE tc.archived_at IS NOT NULL AND (?1 IS NULL OR tc.archived_at >= ?1)
             ORDER BY tc.archived_at",
        )
        .map_err(|e| e.to_string())?;

    let query_rows: Vec<(i64, String, String, Option<String>, String)> = stmt
        .query_map(params![cutoff], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut ids = Vec::new();
    let mut rows = Vec::new();
    let mut blocked = Vec::new();
    for (id, client, code, description, archived_at) in query_rows {
        let default_count = count(conn, "SELECT COUNT(*) FROM clients WHERE default_tracking_code_id = ?1", id)?;
        let entry_count = count(conn, "SELECT COUNT(*) FROM time_entries WHERE tracking_code_id = ?1", id)?;
        if default_count > 0 || entry_count > 0 {
            let mut reasons = Vec::new();
            if default_count > 0 {
                reasons.push("it's set as a client's default category".to_string());
            }
            if entry_count > 0 {
                reasons.push(format!(
                    "{entry_count} time entr{} still reference it",
                    if entry_count == 1 { "y" } else { "ies" }
                ));
            }
            blocked.push(format!("{client} / {code}: {}", reasons.join(" and ")));
            continue;
        }
        ids.push(id);
        rows.push(vec![id.to_string(), client, code, description.unwrap_or_default(), archived_at]);
    }

    let path = unique_path(output_folder, &format!("{}_Category_Archive_Backup_Purge.csv", today()));
    write_csv(&path, &["id", "client", "category", "description", "archived_at"], &rows)?;

    let count = ids.len() as i64;
    delete_by_ids(conn, "tracking_codes", &ids)?;

    Ok(PurgeResult { count, backup_path: path.to_string_lossy().to_string(), blocked })
}

/// Backs up everything, then deletes every client, contract, contract-rate, category,
/// and time entry in the database — leaving the user's own profile/settings intact.
/// There is no dependency ordering to worry about here (unlike the targeted purge
/// functions above) since this clears every table together.
pub fn purge_all_data(conn: &Connection, output_folder: &str) -> DomainResult<super::backup::PurgeAllResult> {
    let backups = super::backup::backup_all_data(conn, output_folder)?;

    conn.execute("DELETE FROM time_entries", []).map_err(|e| e.to_string())?;
    conn.execute("UPDATE clients SET default_tracking_code_id = NULL", []).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM tracking_codes", []).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM contract_rates", []).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM contracts", []).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM clients", []).map_err(|e| e.to_string())?;

    Ok(super::backup::PurgeAllResult { backups })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::contracts::{archive_client, archive_contract, create_client, create_contract};
    use crate::domain::time_entries::{create_manual_entry, delete_entry, list_deleted_entries};
    use crate::domain::tracking_codes::{archive_tracking_code, create_tracking_code};
    use std::fs;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("timetracker_test_purge_{}_{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn purges_deleted_entries_after_writing_backup() {
        let dir = temp_dir("entries");
        let conn = crate::db::open(&dir.join("db.sqlite")).expect("open db");

        let client_id = create_client(&conn, "Purge Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Purge Contract", "USD", 25.0).expect("create contract");
        let entry_id = create_manual_entry(
            &conn,
            contract_id,
            "2026-08-01T09:00:00Z",
            "2026-08-01T10:00:00Z",
            Some("to be purged"),
            None,
        )
        .expect("create entry");
        delete_entry(&conn, entry_id).expect("soft delete");

        let result = purge_deleted_entries(&conn, None, dir.to_str().unwrap()).expect("purge");
        assert_eq!(result.count, 1);
        assert!(result.blocked.is_empty());
        assert!(Path::new(&result.backup_path).exists(), "backup file should exist");

        let backup_contents = fs::read_to_string(&result.backup_path).expect("read backup");
        assert!(backup_contents.contains("to be purged"));
        assert!(backup_contents.contains("Purge Client"));

        let remaining = list_deleted_entries(&conn).expect("list deleted");
        assert!(remaining.is_empty(), "entry should be gone from the live database after purge");
    }

    #[test]
    fn reports_blocked_contract_instead_of_failing_outright() {
        let dir = temp_dir("contract_blocked");
        let conn = crate::db::open(&dir.join("db.sqlite")).expect("open db");

        let client_id = create_client(&conn, "Blocked Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Blocked Contract", "USD", 25.0).expect("create contract");
        create_manual_entry(&conn, contract_id, "2026-08-01T09:00:00Z", "2026-08-01T10:00:00Z", None, None)
            .expect("create entry");
        archive_contract(&conn, contract_id).expect("archive contract");

        let result = purge_archived_contracts(&conn, None, dir.to_str().unwrap()).expect("purge should not error");
        assert_eq!(result.count, 0, "the blocked contract should not have been purged");
        assert_eq!(result.blocked.len(), 1);
        assert!(result.blocked[0].contains("Blocked Contract"), "unexpected message: {}", result.blocked[0]);

        let remaining: i64 = conn
            .query_row("SELECT COUNT(*) FROM contracts WHERE id = ?1", params![contract_id], |r| r.get(0))
            .expect("count");
        assert_eq!(remaining, 1, "blocked contract should still exist");
    }

    #[test]
    fn purges_archived_contract_with_no_referencing_entries() {
        let dir = temp_dir("contract_ok");
        let conn = crate::db::open(&dir.join("db.sqlite")).expect("open db");

        let client_id = create_client(&conn, "Clean Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Clean Contract", "USD", 25.0).expect("create contract");
        archive_contract(&conn, contract_id).expect("archive contract");

        let result = purge_archived_contracts(&conn, None, dir.to_str().unwrap()).expect("purge");
        assert_eq!(result.count, 1);
        assert!(result.blocked.is_empty());
        assert!(Path::new(&result.backup_path).exists());

        let remaining: i64 = conn
            .query_row("SELECT COUNT(*) FROM contracts WHERE id = ?1", params![contract_id], |r| r.get(0))
            .expect("count");
        assert_eq!(remaining, 0);
    }

    #[test]
    fn purges_archived_client_only_after_its_contracts_and_categories_are_gone() {
        let dir = temp_dir("client_chain");
        let conn = crate::db::open(&dir.join("db.sqlite")).expect("open db");

        let client_id = create_client(&conn, "Chain Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Chain Contract", "USD", 25.0).expect("create contract");
        let category_id = create_tracking_code(&conn, client_id, "CODE", None).expect("create category");
        archive_client(&conn, client_id).expect("archive client");

        // Blocked while the (non-archived) contract and category still exist.
        let result = purge_archived_clients(&conn, None, dir.to_str().unwrap()).expect("purge should not error");
        assert_eq!(result.count, 0);
        assert_eq!(result.blocked.len(), 1);
        assert!(result.blocked[0].contains("contract"), "unexpected message: {}", result.blocked[0]);
        assert!(result.blocked[0].contains("categor"), "unexpected message: {}", result.blocked[0]);

        // Clear the dependents the same way a real user would: archive + purge each.
        archive_contract(&conn, contract_id).expect("archive contract");
        purge_archived_contracts(&conn, None, dir.to_str().unwrap()).expect("purge contract");
        archive_tracking_code(&conn, category_id).expect("archive category");
        purge_archived_categories(&conn, None, dir.to_str().unwrap()).expect("purge category");

        let result = purge_archived_clients(&conn, None, dir.to_str().unwrap()).expect("purge client");
        assert_eq!(result.count, 1, "blocked: {:?}", result.blocked);
        assert!(result.blocked.is_empty());
    }

    #[test]
    fn cutoff_only_purges_matching_rows() {
        let dir = temp_dir("cutoff");
        let conn = crate::db::open(&dir.join("db.sqlite")).expect("open db");

        let client_id = create_client(&conn, "Cutoff Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Cutoff Contract", "USD", 25.0).expect("create contract");
        let old_entry = create_manual_entry(&conn, contract_id, "2020-01-01T09:00:00Z", "2020-01-01T10:00:00Z", None, None)
            .expect("create old entry");
        let new_entry = create_manual_entry(&conn, contract_id, "2026-08-01T09:00:00Z", "2026-08-01T10:00:00Z", None, None)
            .expect("create new entry");

        conn.execute(
            "UPDATE time_entries SET deleted_at = '2020-06-01T00:00:00Z' WHERE id = ?1",
            params![old_entry],
        )
        .expect("backdate old delete");
        delete_entry(&conn, new_entry).expect("soft delete new entry");

        // Only entries deleted on/after 2025-01-01 should be purged.
        let result = purge_deleted_entries(&conn, Some("2025-01-01T00:00:00Z"), dir.to_str().unwrap()).expect("purge");
        assert_eq!(result.count, 1);

        let remaining = list_deleted_entries(&conn).expect("list deleted");
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, old_entry, "the pre-cutoff entry should still be in Trash");
    }

    #[test]
    fn purge_all_data_clears_everything_but_keeps_user_profile() {
        let dir = temp_dir("purge_all");
        let conn = crate::db::open(&dir.join("db.sqlite")).expect("open db");

        crate::domain::user_profile::save_user_profile(
            &conn,
            &crate::domain::user_profile::UserProfile {
                first_name: Some("Jesse".to_string()),
                last_name: None,
                full_name: Some("Jesse Example".to_string()),
                email: None,
                output_folder: Some(dir.to_str().unwrap().to_string()),
                output_type: "xlsx".to_string(),
                window_width: None,
                window_height: None,
                window_x: None,
                window_y: None,
                default_start_page: "timer".to_string(),
                launch_position: "default".to_string(),
                theme: "system".to_string(),
                custom_bg: None,
                custom_text: None,
                custom_button_bg: None,
                custom_button_text: None,
            },
        )
        .expect("save profile");

        let client_id = create_client(&conn, "Wipe Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Wipe Contract", "USD", 25.0).expect("create contract");
        create_tracking_code(&conn, client_id, "CODE", None).expect("create category");
        create_manual_entry(&conn, contract_id, "2026-08-01T09:00:00Z", "2026-08-01T10:00:00Z", None, None)
            .expect("create entry");

        let result = purge_all_data(&conn, dir.to_str().unwrap()).expect("purge all");
        assert_eq!(result.backups.len(), 5);

        for table in ["clients", "contracts", "contract_rates", "tracking_codes", "time_entries"] {
            let n: i64 = conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0)).expect("count");
            assert_eq!(n, 0, "{table} should be empty after purge_all_data");
        }

        let profile = crate::domain::user_profile::get_user_profile(&conn).expect("get profile").expect("profile exists");
        assert_eq!(profile.first_name.as_deref(), Some("Jesse"), "user profile should survive purge_all_data");
    }
}
