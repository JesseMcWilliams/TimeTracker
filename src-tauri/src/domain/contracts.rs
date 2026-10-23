use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::DomainResult;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Client {
    pub id: i64,
    pub name: String,
    pub notes: Option<String>,
    pub prime: Option<String>,
    pub sub: Option<String>,
    pub external_id: Option<String>,
    pub week_start: String,
    pub week_end: String,
    pub default_tracking_code_id: Option<i64>,
    pub minimum_increment_minutes: Option<i64>,
    pub archived_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contract {
    pub id: i64,
    pub client_id: i64,
    pub name: String,
    pub currency: String,
    pub external_id: Option<String>,
    pub start_date: Option<String>,
    pub notes: Option<String>,
    /// Which end of the resolved timesheet period ("start" or "end") is used for the
    /// date in a generated timesheet's filename. Defaults to "end".
    pub filename_date: String,
    pub archived_at: Option<String>,
    pub current_rate: Option<f64>,
    /// Sum of all `contract_hour_adjustments` rows for this contract. Zero means no
    /// hours budget has been set (not that the budget is literally zero).
    pub total_hours_budgeted: f64,
    /// Sum of `duration_secs` (in hours) across this contract's non-deleted entries,
    /// regardless of date — hours budgets are a lifetime total, not period-scoped.
    pub hours_logged: f64,
}

const CLIENT_COLUMNS: &str = "id, name, notes, prime, sub, external_id, week_start, week_end, \
     default_tracking_code_id, minimum_increment_minutes, archived_at";

fn row_to_client(row: &rusqlite::Row) -> rusqlite::Result<Client> {
    Ok(Client {
        id: row.get(0)?,
        name: row.get(1)?,
        notes: row.get(2)?,
        prime: row.get(3)?,
        sub: row.get(4)?,
        external_id: row.get(5)?,
        week_start: row.get(6)?,
        week_end: row.get(7)?,
        default_tracking_code_id: row.get(8)?,
        minimum_increment_minutes: row.get(9)?,
        archived_at: row.get(10)?,
    })
}

/// New clients default to a 15-minute minimum billable increment; edit the client to
/// change or clear it.
pub fn create_client(conn: &Connection, name: &str, notes: Option<&str>) -> DomainResult<i64> {
    conn.execute(
        "INSERT INTO clients (name, notes, minimum_increment_minutes) VALUES (?1, ?2, 15)",
        params![name, notes],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[allow(clippy::too_many_arguments)]
pub fn update_client(
    conn: &Connection,
    client_id: i64,
    name: &str,
    notes: Option<&str>,
    prime: Option<&str>,
    sub: Option<&str>,
    external_id: Option<&str>,
    week_start: &str,
    week_end: &str,
    default_tracking_code_id: Option<i64>,
    minimum_increment_minutes: Option<i64>,
) -> DomainResult<()> {
    conn.execute(
        "UPDATE clients SET name = ?1, notes = ?2, prime = ?3, sub = ?4, external_id = ?5, week_start = ?6, week_end = ?7,
            default_tracking_code_id = ?8, minimum_increment_minutes = ?9
         WHERE id = ?10",
        params![
            name,
            notes,
            prime,
            sub,
            external_id,
            week_start,
            week_end,
            default_tracking_code_id,
            minimum_increment_minutes,
            client_id
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn archive_client(conn: &Connection, client_id: i64) -> DomainResult<()> {
    conn.execute(
        "UPDATE clients SET archived_at = datetime('now') WHERE id = ?1",
        params![client_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn restore_client(conn: &Connection, client_id: i64) -> DomainResult<()> {
    conn.execute("UPDATE clients SET archived_at = NULL WHERE id = ?1", params![client_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn list_clients(conn: &Connection, include_archived: bool) -> DomainResult<Vec<Client>> {
    let sql = if include_archived {
        format!("SELECT {CLIENT_COLUMNS} FROM clients ORDER BY name")
    } else {
        format!("SELECT {CLIENT_COLUMNS} FROM clients WHERE archived_at IS NULL ORDER BY name")
    };
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], row_to_client).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn create_contract(
    conn: &Connection,
    client_id: i64,
    name: &str,
    currency: &str,
    initial_hourly_rate: f64,
) -> DomainResult<i64> {
    conn.execute(
        "INSERT INTO contracts (client_id, name, currency) VALUES (?1, ?2, ?3)",
        params![client_id, name, currency],
    )
    .map_err(|e| e.to_string())?;
    let contract_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO contract_rates (contract_id, hourly_rate, effective_from) VALUES (?1, ?2, datetime('now'))",
        params![contract_id, initial_hourly_rate],
    )
    .map_err(|e| e.to_string())?;

    Ok(contract_id)
}

/// Records a new effective rate for a contract. Past time entries already have their
/// rate snapshotted and are unaffected — this only changes what future entries will use.
pub fn update_contract_rate(conn: &Connection, contract_id: i64, new_hourly_rate: f64) -> DomainResult<()> {
    conn.execute(
        "INSERT INTO contract_rates (contract_id, hourly_rate, effective_from) VALUES (?1, ?2, datetime('now'))",
        params![contract_id, new_hourly_rate],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn update_contract_details(
    conn: &Connection,
    contract_id: i64,
    name: &str,
    currency: &str,
    external_id: Option<&str>,
    start_date: Option<&str>,
    notes: Option<&str>,
    filename_date: &str,
) -> DomainResult<()> {
    conn.execute(
        "UPDATE contracts SET name = ?1, currency = ?2, external_id = ?3, start_date = ?4, notes = ?5,
            filename_date = ?6
         WHERE id = ?7",
        params![name, currency, external_id, start_date, notes, filename_date, contract_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// The minimum billable increment (in minutes) for the client that owns `contract_id`,
/// or `None` if that client has no minimum set (no duration rounding applies).
pub fn minimum_increment_for_contract(conn: &Connection, contract_id: i64) -> DomainResult<Option<i64>> {
    conn.query_row(
        "SELECT cl.minimum_increment_minutes FROM contracts c JOIN clients cl ON cl.id = c.client_id
         WHERE c.id = ?1",
        params![contract_id],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

/// Adds (or, with a negative `hours_delta`, removes) hours from a contract's lifetime
/// hours budget. The running total is the sum of every adjustment ever recorded,
/// mirroring how rate changes are tracked. See `set_contract_hours` to state a total
/// directly instead of working out the difference yourself.
pub fn add_contract_hours(
    conn: &Connection,
    contract_id: i64,
    hours_delta: f64,
    note: Option<&str>,
) -> DomainResult<()> {
    conn.execute(
        "INSERT INTO contract_hour_adjustments (contract_id, hours_delta, note) VALUES (?1, ?2, ?3)",
        params![contract_id, hours_delta, note],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Hours budgets smaller than this are treated as zero. Budgets are negotiated in
/// quarter-hours at the finest, so anything under a thousandth of an hour is floating
/// point noise from summing the log rather than a difference anyone meant.
const HOURS_EPSILON: f64 = 0.001;

/// Makes a contract's hours budget equal `total_hours`, by recording the adjustment that
/// closes the gap between the current total and the one asked for.
///
/// The log stays append-only — nothing already recorded is rewritten — so the history
/// still shows how the budget reached its current value, and `total_hours_budgeted`
/// keeps its single definition as the sum of every adjustment. Setting the total to what
/// it already is records nothing rather than a zero-sized entry.
pub fn set_contract_hours(
    conn: &Connection,
    contract_id: i64,
    total_hours: f64,
    note: Option<&str>,
) -> DomainResult<()> {
    let current = total_hours_budgeted(conn, contract_id)?;
    let delta = total_hours - current;
    if delta.abs() < HOURS_EPSILON {
        return Ok(());
    }

    // Without a note of their own, record what was asked for: the log otherwise shows a
    // bare delta, which reads as arbitrary next to the total it was meant to produce.
    let default_note = format!("Set total to {total_hours}");
    let note = note.filter(|n| !n.trim().is_empty()).unwrap_or(&default_note);
    add_contract_hours(conn, contract_id, delta, Some(note))
}

pub fn total_hours_budgeted(conn: &Connection, contract_id: i64) -> DomainResult<f64> {
    conn.query_row(
        "SELECT COALESCE(SUM(hours_delta), 0) FROM contract_hour_adjustments WHERE contract_id = ?1",
        params![contract_id],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

/// Lifetime hours logged against a contract, across all entries regardless of date —
/// hours budgets aren't period-scoped, so "remaining" compares against everything ever
/// logged, not just a report window.
pub fn hours_logged(conn: &Connection, contract_id: i64) -> DomainResult<f64> {
    conn.query_row(
        "SELECT COALESCE(SUM(duration_secs), 0) / 3600.0 FROM time_entries
         WHERE contract_id = ?1 AND deleted_at IS NULL",
        params![contract_id],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

pub fn current_rate(conn: &Connection, contract_id: i64) -> DomainResult<f64> {
    conn.query_row(
        "SELECT hourly_rate FROM contract_rates
         WHERE contract_id = ?1
         ORDER BY effective_from DESC, id DESC
         LIMIT 1",
        params![contract_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())?
    .ok_or_else(|| format!("contract {contract_id} has no rate on record"))
}

pub fn list_contracts(conn: &Connection, include_archived: bool) -> DomainResult<Vec<Contract>> {
    let sql = if include_archived {
        "SELECT id, client_id, name, currency, external_id, start_date, notes, filename_date, archived_at
         FROM contracts ORDER BY name"
    } else {
        "SELECT id, client_id, name, currency, external_id, start_date, notes, filename_date, archived_at
         FROM contracts WHERE archived_at IS NULL ORDER BY name"
    };
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let contracts = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, Option<String>>(8)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    contracts
        .into_iter()
        .map(|(id, client_id, name, currency, external_id, start_date, notes, filename_date, archived_at)| {
            Ok(Contract {
                id,
                client_id,
                name,
                currency,
                external_id,
                start_date,
                notes,
                filename_date,
                archived_at,
                current_rate: current_rate(conn, id).ok(),
                total_hours_budgeted: total_hours_budgeted(conn, id)?,
                hours_logged: hours_logged(conn, id)?,
            })
        })
        .collect()
}

pub fn archive_contract(conn: &Connection, contract_id: i64) -> DomainResult<()> {
    conn.execute(
        "UPDATE contracts SET archived_at = datetime('now') WHERE id = ?1",
        params![contract_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn restore_contract(conn: &Connection, contract_id: i64) -> DomainResult<()> {
    conn.execute(
        "UPDATE contracts SET archived_at = NULL WHERE id = ?1",
        params![contract_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::time_entries::create_manual_entry;
    use std::fs;

    fn test_db(name: &str) -> Connection {
        let pid = std::process::id();
        let db_path = std::env::temp_dir().join(format!("timetracker_test_contracts_{pid}_{name}.sqlite"));
        let _ = fs::remove_file(&db_path);
        crate::db::open(&db_path).expect("open db")
    }

    fn adjustment_count(conn: &Connection, contract_id: i64) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM contract_hour_adjustments WHERE contract_id = ?1",
            params![contract_id],
            |row| row.get(0),
        )
        .expect("count adjustments")
    }

    #[test]
    fn setting_a_total_on_a_contract_with_no_budget_records_the_whole_amount() {
        let conn = test_db("set_from_zero");
        let client_id = create_client(&conn, "Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Contract", "USD", 50.0).expect("create contract");

        set_contract_hours(&conn, contract_id, 235.25, None).expect("set total");

        assert_eq!(total_hours_budgeted(&conn, contract_id).expect("query"), 235.25);
    }

    /// The point of the operation: state the total you want and let the log work out
    /// the difference, rather than adding up the gap by hand.
    #[test]
    fn setting_a_total_records_only_the_difference_from_the_current_one() {
        let conn = test_db("set_records_delta");
        let client_id = create_client(&conn, "Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Contract", "USD", 50.0).expect("create contract");
        add_contract_hours(&conn, contract_id, 200.0, Some("initial")).expect("add hours");

        set_contract_hours(&conn, contract_id, 235.25, None).expect("set total");

        assert_eq!(total_hours_budgeted(&conn, contract_id).expect("query"), 235.25);
        let recorded: f64 = conn
            .query_row(
                "SELECT hours_delta FROM contract_hour_adjustments WHERE contract_id = ?1 ORDER BY id DESC LIMIT 1",
                params![contract_id],
                |row| row.get(0),
            )
            .expect("read last adjustment");
        assert_eq!(recorded, 35.25, "only the gap is recorded, not the whole total");
    }

    /// Append-only: setting a total must never rewrite or remove what came before, or
    /// the history stops explaining how the budget got where it is.
    #[test]
    fn setting_a_total_leaves_the_earlier_adjustments_in_place() {
        let conn = test_db("set_keeps_history");
        let client_id = create_client(&conn, "Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Contract", "USD", 50.0).expect("create contract");
        add_contract_hours(&conn, contract_id, 51.5, None).expect("add hours");
        add_contract_hours(&conn, contract_id, 172.5, None).expect("add hours");

        set_contract_hours(&conn, contract_id, 100.0, None).expect("set total");

        assert_eq!(adjustment_count(&conn, contract_id), 3, "the two earlier rows survive");
        assert_eq!(total_hours_budgeted(&conn, contract_id).expect("query"), 100.0);
    }

    #[test]
    fn setting_a_lower_total_records_a_negative_adjustment() {
        let conn = test_db("set_lower");
        let client_id = create_client(&conn, "Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Contract", "USD", 50.0).expect("create contract");
        add_contract_hours(&conn, contract_id, 100.0, None).expect("add hours");

        set_contract_hours(&conn, contract_id, 40.0, None).expect("set total");

        assert_eq!(total_hours_budgeted(&conn, contract_id).expect("query"), 40.0);
    }

    #[test]
    fn setting_the_total_to_what_it_already_is_records_nothing() {
        let conn = test_db("set_noop");
        let client_id = create_client(&conn, "Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Contract", "USD", 50.0).expect("create contract");
        add_contract_hours(&conn, contract_id, 40.0, None).expect("add hours");

        set_contract_hours(&conn, contract_id, 40.0, None).expect("set total");

        assert_eq!(adjustment_count(&conn, contract_id), 1, "no zero-sized row is added");
    }

    #[test]
    fn setting_a_total_to_zero_clears_the_budget() {
        let conn = test_db("set_zero");
        let client_id = create_client(&conn, "Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Contract", "USD", 50.0).expect("create contract");
        add_contract_hours(&conn, contract_id, 40.0, None).expect("add hours");

        set_contract_hours(&conn, contract_id, 0.0, None).expect("set total");

        assert_eq!(total_hours_budgeted(&conn, contract_id).expect("query"), 0.0);
    }

    /// A bare delta in the log reads as arbitrary next to the total it was meant to
    /// produce, so the default note says what was asked for.
    #[test]
    fn a_set_without_a_note_records_what_was_asked_for() {
        let conn = test_db("set_default_note");
        let client_id = create_client(&conn, "Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Contract", "USD", 50.0).expect("create contract");

        set_contract_hours(&conn, contract_id, 235.25, None).expect("set total");

        let note: Option<String> = conn
            .query_row(
                "SELECT note FROM contract_hour_adjustments WHERE contract_id = ?1",
                params![contract_id],
                |row| row.get(0),
            )
            .expect("read note");
        assert_eq!(note.as_deref(), Some("Set total to 235.25"));
    }

    #[test]
    fn a_caller_supplied_note_wins_over_the_default() {
        let conn = test_db("set_custom_note");
        let client_id = create_client(&conn, "Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Contract", "USD", 50.0).expect("create contract");

        set_contract_hours(&conn, contract_id, 10.0, Some("PO amendment 3")).expect("set total");

        let note: Option<String> = conn
            .query_row(
                "SELECT note FROM contract_hour_adjustments WHERE contract_id = ?1",
                params![contract_id],
                |row| row.get(0),
            )
            .expect("read note");
        assert_eq!(note.as_deref(), Some("PO amendment 3"));
    }

    /// Reproduces how the SAIA budget was actually built — three separate additions to
    /// creep up on a figure — and shows the single call that replaces it.
    #[test]
    fn setting_a_total_replaces_adding_up_to_it_by_hand() {
        let conn = test_db("set_vs_creep");
        let client_id = create_client(&conn, "Client", None).expect("create client");

        let by_hand = create_contract(&conn, client_id, "By hand", "USD", 50.0).expect("create contract");
        for delta in [51.5, 172.5, 11.25] {
            add_contract_hours(&conn, by_hand, delta, None).expect("add hours");
        }

        let at_once = create_contract(&conn, client_id, "At once", "USD", 50.0).expect("create contract");
        set_contract_hours(&conn, at_once, 235.25, None).expect("set total");

        assert_eq!(
            total_hours_budgeted(&conn, by_hand).expect("query"),
            total_hours_budgeted(&conn, at_once).expect("query")
        );
        assert_eq!(adjustment_count(&conn, by_hand), 3);
        assert_eq!(adjustment_count(&conn, at_once), 1);
    }

    #[test]
    fn a_contract_with_no_adjustments_has_a_zero_hours_budget() {
        let conn = test_db("no_adjustments");
        let client_id = create_client(&conn, "Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Contract", "USD", 50.0).expect("create contract");

        assert_eq!(total_hours_budgeted(&conn, contract_id).expect("query"), 0.0);
    }

    #[test]
    fn hour_adjustments_add_and_subtract_from_the_running_total() {
        let conn = test_db("adjustments");
        let client_id = create_client(&conn, "Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Contract", "USD", 50.0).expect("create contract");

        add_contract_hours(&conn, contract_id, 40.0, Some("initial budget")).expect("add hours");
        add_contract_hours(&conn, contract_id, 10.0, None).expect("add more hours");
        add_contract_hours(&conn, contract_id, -5.0, Some("correction")).expect("remove hours");

        assert_eq!(total_hours_budgeted(&conn, contract_id).expect("query"), 45.0);
    }

    #[test]
    fn hours_logged_sums_lifetime_entries_regardless_of_date() {
        let conn = test_db("hours_logged");
        let client_id = create_client(&conn, "Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Contract", "USD", 50.0).expect("create contract");

        create_manual_entry(&conn, contract_id, "2026-01-01T09:00:00Z", "2026-01-01T11:00:00Z", None, None)
            .expect("create old entry");
        create_manual_entry(&conn, contract_id, "2026-08-24T09:00:00Z", "2026-08-24T10:30:00Z", None, None)
            .expect("create recent entry");

        assert_eq!(hours_logged(&conn, contract_id).expect("query"), 3.5);
    }
}
