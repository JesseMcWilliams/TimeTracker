use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::contracts::{current_rate, minimum_increment_for_contract};
use super::tracking_codes::validate_tracking_code;
use super::DomainResult;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeEntry {
    pub id: i64,
    pub contract_id: i64,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_secs: Option<i64>,
    pub rate_snapshot: f64,
    pub notes: Option<String>,
    pub source: String,
    pub external_ref: Option<String>,
    pub tracking_code_id: Option<i64>,
    pub tracking_code: Option<String>,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryFilter {
    pub contract_id: Option<i64>,
    pub from: Option<String>,
    pub to: Option<String>,
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn duration_between(started_at: &str, ended_at: &str) -> DomainResult<i64> {
    let start = DateTime::parse_from_rfc3339(started_at).map_err(|e| e.to_string())?;
    let end = DateTime::parse_from_rfc3339(ended_at).map_err(|e| e.to_string())?;
    Ok((end - start).num_seconds())
}

/// Duration for billing purposes: the raw elapsed time, rounded up to the next
/// multiple of the contract's client's minimum billable increment (if any is set).
/// A 22-minute entry for a client with a 15-minute minimum bills as 30 minutes.
fn compute_duration_secs(conn: &Connection, contract_id: i64, started_at: &str, ended_at: &str) -> DomainResult<i64> {
    let raw = duration_between(started_at, ended_at)?;
    let increment_minutes = minimum_increment_for_contract(conn, contract_id)?.unwrap_or(0);
    if increment_minutes <= 0 {
        return Ok(raw);
    }
    let increment_secs = increment_minutes * 60;
    Ok(((raw + increment_secs - 1) / increment_secs) * increment_secs)
}

fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<TimeEntry> {
    Ok(TimeEntry {
        id: row.get(0)?,
        contract_id: row.get(1)?,
        started_at: row.get(2)?,
        ended_at: row.get(3)?,
        duration_secs: row.get(4)?,
        rate_snapshot: row.get(5)?,
        notes: row.get(6)?,
        source: row.get(7)?,
        external_ref: row.get(8)?,
        tracking_code_id: row.get(9)?,
        tracking_code: row.get(10)?,
        deleted_at: row.get(11)?,
    })
}

const SELECT_COLUMNS: &str = "te.id, te.contract_id, te.started_at, te.ended_at, te.duration_secs, \
     te.rate_snapshot, te.notes, te.source, te.external_ref, te.tracking_code_id, tc.code, te.deleted_at";
const FROM_CLAUSE: &str = "FROM time_entries te LEFT JOIN tracking_codes tc ON tc.id = te.tracking_code_id";

/// Starts a running timer against `contract_id`. Concurrent timers across different
/// contracts are allowed (work can legitimately span multiple clients at once) — callers
/// should check `get_active_timers` first and warn the user, but this function itself
/// does not block on an already-running timer.
pub fn start_timer(
    conn: &Connection,
    contract_id: i64,
    notes: Option<&str>,
    tracking_code_id: Option<i64>,
) -> DomainResult<i64> {
    validate_tracking_code(conn, contract_id, tracking_code_id)?;
    let rate = current_rate(conn, contract_id)?;
    let started_at = now_iso();
    conn.execute(
        "INSERT INTO time_entries (contract_id, started_at, rate_snapshot, notes, source, tracking_code_id)
         VALUES (?1, ?2, ?3, ?4, 'timer', ?5)",
        params![contract_id, started_at, rate, notes, tracking_code_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

pub fn stop_timer(conn: &Connection, entry_id: i64) -> DomainResult<()> {
    let (contract_id, started_at): (i64, String) = conn
        .query_row(
            "SELECT contract_id, started_at FROM time_entries WHERE id = ?1 AND ended_at IS NULL",
            params![entry_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("no running timer with id {entry_id}: {e}"))?;

    let ended_at = now_iso();
    let duration_secs = compute_duration_secs(conn, contract_id, &started_at, &ended_at)?;

    conn.execute(
        "UPDATE time_entries
         SET ended_at = ?1, duration_secs = ?2, updated_at = datetime('now')
         WHERE id = ?3",
        params![ended_at, duration_secs, entry_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_active_timers(conn: &Connection) -> DomainResult<Vec<TimeEntry>> {
    let sql = format!(
        "SELECT {SELECT_COLUMNS} {FROM_CLAUSE} WHERE te.ended_at IS NULL AND te.deleted_at IS NULL ORDER BY te.started_at"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], row_to_entry).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn create_manual_entry(
    conn: &Connection,
    contract_id: i64,
    started_at: &str,
    ended_at: &str,
    notes: Option<&str>,
    tracking_code_id: Option<i64>,
) -> DomainResult<i64> {
    validate_tracking_code(conn, contract_id, tracking_code_id)?;
    let rate = current_rate(conn, contract_id)?;
    let duration_secs = compute_duration_secs(conn, contract_id, started_at, ended_at)?;

    conn.execute(
        "INSERT INTO time_entries (contract_id, started_at, ended_at, duration_secs, rate_snapshot, notes, source, tracking_code_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'manual', ?7)",
        params![contract_id, started_at, ended_at, duration_secs, rate, notes, tracking_code_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

/// Edits an existing entry's time range/notes/tracking code. `rate_snapshot`,
/// `contract_id`, and `source` are intentionally not editable here — past billing
/// figures must not shift silently. Deleting and recreating the entry is the correct
/// path for those changes.
pub fn update_entry(
    conn: &Connection,
    entry_id: i64,
    started_at: &str,
    ended_at: &str,
    notes: Option<&str>,
    tracking_code_id: Option<i64>,
) -> DomainResult<()> {
    let contract_id: i64 = conn
        .query_row(
            "SELECT contract_id FROM time_entries WHERE id = ?1",
            params![entry_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    validate_tracking_code(conn, contract_id, tracking_code_id)?;

    let duration_secs = compute_duration_secs(conn, contract_id, started_at, ended_at)?;
    conn.execute(
        "UPDATE time_entries
         SET started_at = ?1, ended_at = ?2, duration_secs = ?3, notes = ?4, tracking_code_id = ?5, updated_at = datetime('now')
         WHERE id = ?6",
        params![started_at, ended_at, duration_secs, notes, tracking_code_id, entry_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Updates just notes/category, leaving the time range untouched — used to edit a
/// still-running timer, which has no end time yet to pass through `update_entry`.
pub fn update_entry_metadata(
    conn: &Connection,
    entry_id: i64,
    notes: Option<&str>,
    tracking_code_id: Option<i64>,
) -> DomainResult<()> {
    let contract_id: i64 = conn
        .query_row(
            "SELECT contract_id FROM time_entries WHERE id = ?1",
            params![entry_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    validate_tracking_code(conn, contract_id, tracking_code_id)?;

    conn.execute(
        "UPDATE time_entries SET notes = ?1, tracking_code_id = ?2, updated_at = datetime('now') WHERE id = ?3",
        params![notes, tracking_code_id, entry_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Soft-deletes an entry (marks it `deleted_at`) so it drops out of normal listings
/// but can still be recovered via `restore_entry` / `list_deleted_entries`.
pub fn delete_entry(conn: &Connection, entry_id: i64) -> DomainResult<()> {
    conn.execute(
        "UPDATE time_entries SET deleted_at = datetime('now') WHERE id = ?1",
        params![entry_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn restore_entry(conn: &Connection, entry_id: i64) -> DomainResult<()> {
    conn.execute(
        "UPDATE time_entries SET deleted_at = NULL WHERE id = ?1",
        params![entry_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn list_deleted_entries(conn: &Connection) -> DomainResult<Vec<TimeEntry>> {
    let sql =
        format!("SELECT {SELECT_COLUMNS} {FROM_CLAUSE} WHERE te.deleted_at IS NOT NULL ORDER BY te.started_at DESC");
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], row_to_entry).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn list_entries(conn: &Connection, filter: &EntryFilter) -> DomainResult<Vec<TimeEntry>> {
    let mut sql = format!("SELECT {SELECT_COLUMNS} {FROM_CLAUSE} WHERE te.deleted_at IS NULL");
    let mut sql_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(contract_id) = filter.contract_id {
        sql.push_str(" AND te.contract_id = ?");
        sql_params.push(Box::new(contract_id));
    }
    if let Some(from) = &filter.from {
        sql.push_str(" AND te.started_at >= ?");
        sql_params.push(Box::new(from.clone()));
    }
    if let Some(to) = &filter.to {
        sql.push_str(" AND te.started_at <= ?");
        sql_params.push(Box::new(to.clone()));
    }
    sql.push_str(" ORDER BY te.started_at DESC");

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = sql_params.iter().map(|p| p.as_ref()).collect();
    let rows = stmt
        .query_map(param_refs.as_slice(), row_to_entry)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::contracts::{create_client, create_contract, update_client};
    use std::fs;

    fn temp_db(name: &str) -> Connection {
        let path = std::env::temp_dir().join(format!("timetracker_test_te_{}_{name}.sqlite", std::process::id()));
        let _ = fs::remove_file(&path);
        crate::db::open(&path).expect("open db")
    }

    #[test]
    fn rounds_duration_up_to_clients_minimum_increment() {
        let conn = temp_db("rounding");
        let client_id = create_client(&conn, "Rounding Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Rounding Contract", "USD", 60.0).expect("create contract");

        // Set a 15-minute minimum for this client.
        update_client(&conn, client_id, "Rounding Client", None, None, None, None, "monday", "sunday", None, Some(15))
            .expect("set minimum increment");

        // 22 real minutes should bill as 30 (next 15-minute multiple).
        let entry_id = create_manual_entry(
            &conn,
            contract_id,
            "2026-08-01T09:00:00Z",
            "2026-08-01T09:22:00Z",
            None,
            None,
        )
        .expect("create entry");
        let entries = list_entries(&conn, &EntryFilter { contract_id: Some(contract_id), ..Default::default() })
            .expect("list entries");
        let entry = entries.iter().find(|e| e.id == entry_id).expect("entry present");
        assert_eq!(entry.duration_secs, Some(30 * 60), "22 minutes should round up to 30 with a 15-minute minimum");

        // An exact multiple (30 minutes) should not round up further.
        let exact_id =
            create_manual_entry(&conn, contract_id, "2026-08-01T10:00:00Z", "2026-08-01T10:30:00Z", None, None)
                .expect("create exact entry");
        let entries = list_entries(&conn, &EntryFilter { contract_id: Some(contract_id), ..Default::default() })
            .expect("list entries");
        let exact = entries.iter().find(|e| e.id == exact_id).expect("entry present");
        assert_eq!(exact.duration_secs, Some(30 * 60), "an exact multiple should stay unchanged");
    }

    #[test]
    fn no_rounding_when_client_has_no_minimum_set() {
        let conn = temp_db("no_rounding");
        let client_id = create_client(&conn, "Plain Client", None).expect("create client");
        // New clients default to a 15-minute minimum; clear it to exercise the no-rounding path.
        conn.execute(
            "UPDATE clients SET minimum_increment_minutes = NULL WHERE id = ?1",
            params![client_id],
        )
        .expect("clear minimum increment");
        let contract_id = create_contract(&conn, client_id, "Plain Contract", "USD", 60.0).expect("create contract");

        let entry_id =
            create_manual_entry(&conn, contract_id, "2026-08-01T09:00:00Z", "2026-08-01T09:22:00Z", None, None)
                .expect("create entry");
        let entries = list_entries(&conn, &EntryFilter { contract_id: Some(contract_id), ..Default::default() })
            .expect("list entries");
        let entry = entries.iter().find(|e| e.id == entry_id).expect("entry present");
        assert_eq!(entry.duration_secs, Some(22 * 60), "no minimum set means no rounding");
    }
}
