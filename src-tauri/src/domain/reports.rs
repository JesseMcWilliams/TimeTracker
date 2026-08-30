use chrono::{NaiveDate, Weekday};
use rusqlite::{params, Connection};
use serde::Serialize;

use super::period::{month_range, to_rfc3339_bounds, week_range};
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
pub struct Report {
    pub from: String,
    pub to: String,
    pub total_secs: i64,
    pub total_amount: f64,
    pub clients: Vec<ClientBreakdown>,
}

/// Aggregate reports use a fixed Monday-Sunday week regardless of any individual
/// client's own week settings, since one report mixes entries across many clients and
/// needs one consistent window. Per-client week boundaries are honored instead in
/// per-contract timesheet generation (see `timesheets::generate_timesheets`).
pub fn generate_report(conn: &Connection, period: &str, reference_date: &str) -> DomainResult<Report> {
    let reference: NaiveDate = reference_date.parse().map_err(|e| format!("invalid date: {e}"))?;
    let (start, end) = if period == "week" {
        week_range(reference, Weekday::Mon, Weekday::Sun)
    } else {
        month_range(reference)
    };
    let (from, to) = to_rfc3339_bounds(start, end);

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

    Ok(Report {
        from: start.format("%Y-%m-%d").to_string(),
        to: end.format("%Y-%m-%d").to_string(),
        total_secs,
        total_amount,
        clients,
    })
}
