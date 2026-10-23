use chrono::{DateTime, Local, Offset, Utc};
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
    /// Inclusive `YYYY-MM-DD` bounds on the day an entry belongs to, not on the instant
    /// it started — "today" means the caller's today, which is what `local_date` records.
    pub from: Option<String>,
    pub to: Option<String>,
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

/// The calendar day an entry belongs to, plus the offset east of UTC that decided it.
///
/// Both are stored on the row and never recomputed: `started_at` is a UTC instant, and
/// turning it into a day is a question only the timezone at the moment of recording can
/// answer. Deriving it later from whoever happens to be looking lets an entry drift into
/// a different week just because the app was opened from a different timezone.
pub(crate) fn local_day_of(started_at: &str) -> DomainResult<(String, i32)> {
    let instant = DateTime::parse_from_rfc3339(started_at).map_err(|e| format!("invalid timestamp: {e}"))?;
    let local = instant.with_timezone(&Local);
    let offset_minutes = local.offset().fix().local_minus_utc() / 60;
    Ok((local.date_naive().format("%Y-%m-%d").to_string(), offset_minutes))
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
    let (local_date, utc_offset_minutes) = local_day_of(&started_at)?;
    conn.execute(
        "INSERT INTO time_entries (contract_id, started_at, rate_snapshot, notes, source, tracking_code_id,
            local_date, utc_offset_minutes)
         VALUES (?1, ?2, ?3, ?4, 'timer', ?5, ?6, ?7)",
        params![contract_id, started_at, rate, notes, tracking_code_id, local_date, utc_offset_minutes],
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
    create_entry_with_source(conn, contract_id, started_at, ended_at, notes, tracking_code_id, "manual")
}

/// Same as `create_manual_entry`, but lets the caller record a different `source` than
/// 'manual' — used by bulk import so imported rows are distinguishable from entries
/// typed in by hand (e.g. on the Entries page's Source column).
pub(crate) fn create_entry_with_source(
    conn: &Connection,
    contract_id: i64,
    started_at: &str,
    ended_at: &str,
    notes: Option<&str>,
    tracking_code_id: Option<i64>,
    source: &str,
) -> DomainResult<i64> {
    validate_tracking_code(conn, contract_id, tracking_code_id)?;
    let rate = current_rate(conn, contract_id)?;
    let duration_secs = compute_duration_secs(conn, contract_id, started_at, ended_at)?;
    let (local_date, utc_offset_minutes) = local_day_of(started_at)?;

    conn.execute(
        "INSERT INTO time_entries (contract_id, started_at, ended_at, duration_secs, rate_snapshot, notes, source,
            tracking_code_id, local_date, utc_offset_minutes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            contract_id,
            started_at,
            ended_at,
            duration_secs,
            rate,
            notes,
            source,
            tracking_code_id,
            local_date,
            utc_offset_minutes
        ],
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
    // Editing the start time can move the entry to a different day, so the stored day is
    // recomputed here rather than left pointing at where the entry used to be.
    let (local_date, utc_offset_minutes) = local_day_of(started_at)?;
    conn.execute(
        "UPDATE time_entries
         SET started_at = ?1, ended_at = ?2, duration_secs = ?3, notes = ?4, tracking_code_id = ?5,
             local_date = ?6, utc_offset_minutes = ?7, updated_at = datetime('now')
         WHERE id = ?8",
        params![
            started_at,
            ended_at,
            duration_secs,
            notes,
            tracking_code_id,
            local_date,
            utc_offset_minutes,
            entry_id
        ],
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
        sql.push_str(" AND te.local_date >= ?");
        sql_params.push(Box::new(from.clone()));
    }
    if let Some(to) = &filter.to {
        sql.push_str(" AND te.local_date <= ?");
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
    use chrono::Duration;
    use std::fs;

    fn temp_db(name: &str) -> Connection {
        let path = std::env::temp_dir().join(format!("timetracker_test_te_{}_{name}.sqlite", std::process::id()));
        let _ = fs::remove_file(&path);
        crate::db::open(&path).expect("open db")
    }

    /// The local day of a UTC instant, as this machine reckons it. A test asserting a
    /// literal date would only hold in one timezone, so these assert the relationship
    /// between the stored instant and the stored day instead.
    fn expected_local_day(iso: &str) -> (String, i32) {
        let local = DateTime::parse_from_rfc3339(iso).expect("valid timestamp").with_timezone(&Local);
        (local.date_naive().format("%Y-%m-%d").to_string(), local.offset().fix().local_minus_utc() / 60)
    }

    fn stored_day(conn: &Connection, entry_id: i64) -> (String, i32) {
        conn.query_row(
            "SELECT local_date, utc_offset_minutes FROM time_entries WHERE id = ?1",
            params![entry_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read stored day")
    }

    #[test]
    fn local_day_of_reports_the_day_and_the_offset_that_decided_it() {
        let iso = "2026-08-24T12:00:00Z";

        assert_eq!(local_day_of(iso).expect("resolve local day"), expected_local_day(iso));
    }

    #[test]
    fn local_day_of_rejects_a_timestamp_it_cannot_parse() {
        assert!(local_day_of("not a timestamp").is_err());
    }

    /// An instant just after UTC midnight is the previous day west of Greenwich and the
    /// same day east of it. Either way the stored day must match what the clock on the
    /// wall said, which is the whole point of storing it.
    #[test]
    fn an_entry_just_after_utc_midnight_is_filed_under_the_local_day() {
        let conn = temp_db("utc_midnight_edge");
        let client_id = create_client(&conn, "Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Contract", "USD", 50.0).expect("create contract");

        let started = "2026-08-24T00:30:00Z";
        let entry_id = create_manual_entry(&conn, contract_id, started, "2026-08-24T01:30:00Z", None, None)
            .expect("create entry");

        let (local_date, offset) = stored_day(&conn, entry_id);
        assert_eq!((local_date.clone(), offset), expected_local_day(started));

        // The two stored values agree: applying the stored offset to the stored instant
        // lands on the stored day.
        let shifted = DateTime::parse_from_rfc3339(started).unwrap().naive_utc() + Duration::minutes(offset as i64);
        assert_eq!(shifted.format("%Y-%m-%d").to_string(), local_date);
    }

    #[test]
    fn a_started_timer_records_the_day_it_started_on() {
        let conn = temp_db("timer_day");
        let client_id = create_client(&conn, "Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Contract", "USD", 50.0).expect("create contract");

        let entry_id = start_timer(&conn, contract_id, None, None).expect("start timer");

        let started_at: String = conn
            .query_row("SELECT started_at FROM time_entries WHERE id = ?1", params![entry_id], |r| r.get(0))
            .expect("read started_at");
        assert_eq!(stored_day(&conn, entry_id), expected_local_day(&started_at));
    }

    /// Moving an entry to another day has to move which period it bills to, otherwise
    /// the stored day still points at where the entry used to be.
    #[test]
    fn editing_the_start_time_moves_the_stored_day() {
        let conn = temp_db("edit_moves_day");
        let client_id = create_client(&conn, "Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Contract", "USD", 50.0).expect("create contract");

        let entry_id = create_manual_entry(&conn, contract_id, "2026-08-24T18:00:00Z", "2026-08-24T19:00:00Z", None, None)
            .expect("create entry");
        let before = stored_day(&conn, entry_id).0;

        let moved = "2026-09-02T18:00:00Z";
        update_entry(&conn, entry_id, moved, "2026-09-02T19:00:00Z", None, None).expect("update entry");

        let after = stored_day(&conn, entry_id).0;
        assert_ne!(after, before);
        assert_eq!(after, expected_local_day(moved).0);
    }

    /// Editing only notes must not disturb the day — update_entry_metadata leaves the
    /// time range alone, so it has no business touching what that range implied.
    #[test]
    fn editing_only_metadata_leaves_the_stored_day_alone() {
        let conn = temp_db("metadata_keeps_day");
        let client_id = create_client(&conn, "Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Contract", "USD", 50.0).expect("create contract");

        let entry_id = create_manual_entry(&conn, contract_id, "2026-08-24T18:00:00Z", "2026-08-24T19:00:00Z", None, None)
            .expect("create entry");
        let before = stored_day(&conn, entry_id);

        update_entry_metadata(&conn, entry_id, Some("a note"), None).expect("update metadata");

        assert_eq!(stored_day(&conn, entry_id), before);
    }

    /// Every entry must carry a day. A NULL would drop out of every period query
    /// silently, which is the failure mode this column exists to remove.
    #[test]
    fn no_entry_is_ever_written_without_a_day() {
        let conn = temp_db("no_null_days");
        let client_id = create_client(&conn, "Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Contract", "USD", 50.0).expect("create contract");

        create_manual_entry(&conn, contract_id, "2026-08-24T18:00:00Z", "2026-08-24T19:00:00Z", None, None)
            .expect("create manual entry");
        start_timer(&conn, contract_id, None, None).expect("start timer");

        let nulls: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM time_entries WHERE local_date IS NULL OR utc_offset_minutes IS NULL",
                [],
                |row| row.get(0),
            )
            .expect("count nulls");
        assert_eq!(nulls, 0);
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
