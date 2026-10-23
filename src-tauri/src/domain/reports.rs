use chrono::{NaiveDate, Weekday};
use rusqlite::{params, Connection};
use serde::Serialize;

use super::period::{resolve_period, to_local_date_bounds};
use super::DomainResult;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractBreakdown {
    pub contract_id: i64,
    pub contract_name: String,
    pub total_secs: i64,
    pub total_amount: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientBreakdown {
    pub client_id: i64,
    pub client_name: String,
    pub total_secs: i64,
    pub total_amount: f64,
    pub contracts: Vec<ContractBreakdown>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractRef {
    pub contract_id: i64,
    pub contract_name: String,
    pub client_id: i64,
    pub client_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub from: String,
    pub to: String,
    pub total_secs: i64,
    pub total_amount: f64,
    pub clients: Vec<ClientBreakdown>,
    pub contracts_without_time: Vec<ContractRef>,
}

/// Aggregate reports use a fixed Monday-Sunday week regardless of any individual
/// client's own week settings, since one report mixes entries across many clients and
/// needs one consistent window. Per-client week boundaries are honored instead in
/// per-contract timesheet generation (see `timesheets::generate_timesheets`). Supports
/// "week", "month", and "year" periods (see `period::resolve_period`).
pub fn generate_report(conn: &Connection, period: &str, reference_date: &str) -> DomainResult<Report> {
    let reference: NaiveDate = reference_date.parse().map_err(|e| format!("invalid date: {e}"))?;
    let (start, end) = resolve_period(period, reference, Weekday::Mon, Weekday::Sun);
    let (from, to) = to_local_date_bounds(start, end);

    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.name, cl.id, cl.name,
                    COALESCE(SUM(te.duration_secs), 0),
                    COALESCE(SUM((te.duration_secs / 3600.0) * te.rate_snapshot), 0)
             FROM time_entries te
             JOIN contracts c ON c.id = te.contract_id
             JOIN clients cl ON cl.id = c.client_id
             WHERE te.deleted_at IS NULL AND te.started_at >= ?1 AND te.started_at <= ?2
             GROUP BY c.id
             ORDER BY cl.name, c.name",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![from, to], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, f64>(5)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut clients: Vec<ClientBreakdown> = Vec::new();
    let mut total_secs = 0i64;
    let mut total_amount = 0f64;

    for (contract_id, contract_name, client_id, client_name, secs, amount) in rows {
        total_secs += secs;
        total_amount += amount;
        if let Some(existing) = clients.iter_mut().find(|c| c.client_id == client_id) {
            existing.total_secs += secs;
            existing.total_amount += amount;
            existing.contracts.push(ContractBreakdown {
                contract_id,
                contract_name,
                total_secs: secs,
                total_amount: amount,
            });
        } else {
            clients.push(ClientBreakdown {
                client_id,
                client_name,
                total_secs: secs,
                total_amount: amount,
                contracts: vec![ContractBreakdown {
                    contract_id,
                    contract_name,
                    total_secs: secs,
                    total_amount: amount,
                }],
            });
        }
    }

    let contracts_without_time = contracts_without_time(conn, &from, &to)?;

    Ok(Report {
        from: start.format("%Y-%m-%d").to_string(),
        to: end.format("%Y-%m-%d").to_string(),
        total_secs,
        total_amount,
        clients,
        contracts_without_time,
    })
}

/// Active contracts with zero time entries in [from, to] — the inner-join query above
/// only ever returns contracts that already have at least one entry, so this is a
/// separate left-join-from-contracts query to surface the rest.
fn contracts_without_time(conn: &Connection, from: &str, to: &str) -> DomainResult<Vec<ContractRef>> {
    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.name, cl.id, cl.name
             FROM contracts c
             JOIN clients cl ON cl.id = c.client_id
             WHERE c.archived_at IS NULL
               AND c.id NOT IN (
                 SELECT DISTINCT te.contract_id FROM time_entries te
                 WHERE te.deleted_at IS NULL AND te.started_at >= ?1 AND te.started_at <= ?2
               )
             ORDER BY cl.name, c.name",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![from, to], |row| {
            Ok(ContractRef {
                contract_id: row.get(0)?,
                contract_name: row.get(1)?,
                client_id: row.get(2)?,
                client_name: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::contracts::{archive_contract, create_client, create_contract};
    use crate::domain::time_entries::create_manual_entry;
    use std::fs;

    fn test_db(name: &str) -> Connection {
        let pid = std::process::id();
        let db_path = std::env::temp_dir().join(format!("timetracker_test_reports_{pid}_{name}.sqlite"));
        let _ = fs::remove_file(&db_path);
        crate::db::open(&db_path).expect("open db")
    }

    #[test]
    fn contracts_with_no_entries_in_period_are_listed_separately() {
        let conn = test_db("no_entries");
        let client_id = create_client(&conn, "Quiet Client", None).expect("create client");
        let worked_contract = create_contract(&conn, client_id, "Worked On", "USD", 50.0).expect("create contract");
        let idle_contract = create_contract(&conn, client_id, "No Time Logged", "USD", 50.0).expect("create contract");

        create_manual_entry(&conn, worked_contract, "2026-08-24T09:00:00Z", "2026-08-24T10:00:00Z", None, None)
            .expect("create entry");

        let report = generate_report(&conn, "week", "2026-08-24").expect("generate report");

        assert_eq!(report.clients.len(), 1, "only the worked contract's client appears in the normal breakdown");
        assert_eq!(report.contracts_without_time.len(), 1);
        assert_eq!(report.contracts_without_time[0].contract_id, idle_contract);
    }

    #[test]
    fn archived_contracts_are_excluded_from_contracts_without_time() {
        let conn = test_db("archived");
        let client_id = create_client(&conn, "Client", None).expect("create client");
        let archived_contract = create_contract(&conn, client_id, "Retired", "USD", 50.0).expect("create contract");
        archive_contract(&conn, archived_contract).expect("archive contract");

        let report = generate_report(&conn, "week", "2026-08-24").expect("generate report");

        assert!(report.contracts_without_time.is_empty(), "archived contracts should not be flagged as idle");
    }
}
