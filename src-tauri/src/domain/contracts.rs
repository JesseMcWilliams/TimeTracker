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
    pub archived_at: Option<String>,
    pub current_rate: Option<f64>,
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
) -> DomainResult<()> {
    conn.execute(
        "UPDATE contracts SET name = ?1, currency = ?2, external_id = ?3, start_date = ?4, notes = ?5 WHERE id = ?6",
        params![name, currency, external_id, start_date, notes, contract_id],
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
        "SELECT id, client_id, name, currency, external_id, start_date, notes, archived_at FROM contracts ORDER BY name"
    } else {
        "SELECT id, client_id, name, currency, external_id, start_date, notes, archived_at FROM contracts
         WHERE archived_at IS NULL ORDER BY name"
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
                row.get::<_, Option<String>>(7)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    contracts
        .into_iter()
        .map(|(id, client_id, name, currency, external_id, start_date, notes, archived_at)| {
            Ok(Contract {
                id,
                client_id,
                name,
                currency,
                external_id,
                start_date,
                notes,
                archived_at,
                current_rate: current_rate(conn, id).ok(),
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
