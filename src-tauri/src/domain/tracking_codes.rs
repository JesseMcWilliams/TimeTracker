use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::DomainResult;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackingCode {
    pub id: i64,
    pub client_id: i64,
    pub code: String,
    pub description: Option<String>,
    pub archived_at: Option<String>,
}

pub fn create_tracking_code(
    conn: &Connection,
    client_id: i64,
    code: &str,
    description: Option<&str>,
) -> DomainResult<i64> {
    conn.execute(
        "INSERT INTO tracking_codes (client_id, code, description) VALUES (?1, ?2, ?3)",
        params![client_id, code, description],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

pub fn list_tracking_codes(
    conn: &Connection,
    client_id: i64,
    include_archived: bool,
) -> DomainResult<Vec<TrackingCode>> {
    let sql = if include_archived {
        "SELECT id, client_id, code, description, archived_at FROM tracking_codes
         WHERE client_id = ?1 ORDER BY code"
    } else {
        "SELECT id, client_id, code, description, archived_at FROM tracking_codes
         WHERE client_id = ?1 AND archived_at IS NULL ORDER BY code"
    };
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![client_id], |row| {
            Ok(TrackingCode {
                id: row.get(0)?,
                client_id: row.get(1)?,
                code: row.get(2)?,
                description: row.get(3)?,
                archived_at: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn archive_tracking_code(conn: &Connection, tracking_code_id: i64) -> DomainResult<()> {
    conn.execute(
        "UPDATE tracking_codes SET archived_at = datetime('now') WHERE id = ?1",
        params![tracking_code_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn restore_tracking_code(conn: &Connection, tracking_code_id: i64) -> DomainResult<()> {
    conn.execute(
        "UPDATE tracking_codes SET archived_at = NULL WHERE id = ?1",
        params![tracking_code_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn list_all_archived(conn: &Connection) -> DomainResult<Vec<TrackingCode>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, client_id, code, description, archived_at FROM tracking_codes
             WHERE archived_at IS NOT NULL ORDER BY client_id, code",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(TrackingCode {
                id: row.get(0)?,
                client_id: row.get(1)?,
                code: row.get(2)?,
                description: row.get(3)?,
                archived_at: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn client_id_for_contract(conn: &Connection, contract_id: i64) -> DomainResult<i64> {
    conn.query_row(
        "SELECT client_id FROM contracts WHERE id = ?1",
        params![contract_id],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

/// A category is always optional — this only checks that, when one *is* given, it
/// actually belongs to the contract's client (referential integrity), not that one is
/// present at all.
pub fn validate_tracking_code(
    conn: &Connection,
    contract_id: i64,
    tracking_code_id: Option<i64>,
) -> DomainResult<()> {
    let Some(id) = tracking_code_id else {
        return Ok(());
    };
    let client_id = client_id_for_contract(conn, contract_id)?;
    let codes = list_tracking_codes(conn, client_id, false)?;
    if codes.iter().any(|c| c.id == id) {
        Ok(())
    } else {
        Err("selected category does not belong to this contract's client".to_string())
    }
}
