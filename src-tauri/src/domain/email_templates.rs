use chrono::NaiveDate;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::period::{resolve_date_range_preset, to_rfc3339_bounds};
use super::DomainResult;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailTemplate {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub email: Option<String>,
    /// `None` means "check every active contract"; `Some(id)` scopes the check to one.
    pub contract_id: Option<i64>,
    pub date_range: String,
}

/// A contract with no time logged in the resolved date range — same shape as
/// `reports::ContractRef`, kept separate so this module doesn't depend on another
/// still-in-flight feature branch's code.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdleContract {
    pub contract_id: i64,
    pub contract_name: String,
    pub client_id: i64,
    pub client_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoTimeEmailPreview {
    pub subject: String,
    pub body: String,
    pub to: Option<String>,
    pub from: String,
    pub to_date: String,
    pub contracts: Vec<IdleContract>,
}

const COLUMNS: &str = "id, name, description, notes, email, contract_id, date_range";

fn row_to_template(row: &rusqlite::Row) -> rusqlite::Result<EmailTemplate> {
    Ok(EmailTemplate {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        notes: row.get(3)?,
        email: row.get(4)?,
        contract_id: row.get(5)?,
        date_range: row.get(6)?,
    })
}

pub fn list_email_templates(conn: &Connection) -> DomainResult<Vec<EmailTemplate>> {
    let mut stmt =
        conn.prepare(&format!("SELECT {COLUMNS} FROM email_templates ORDER BY name")).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], row_to_template).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn get_email_template(conn: &Connection, id: i64) -> DomainResult<EmailTemplate> {
    conn.query_row(&format!("SELECT {COLUMNS} FROM email_templates WHERE id = ?1"), params![id], row_to_template)
        .map_err(|e| format!("email template {id} not found: {e}"))
}

#[allow(clippy::too_many_arguments)]
pub fn create_email_template(
    conn: &Connection,
    name: &str,
    description: Option<&str>,
    notes: Option<&str>,
    email: Option<&str>,
    contract_id: Option<i64>,
    date_range: &str,
) -> DomainResult<i64> {
    conn.execute(
        "INSERT INTO email_templates (name, description, notes, email, contract_id, date_range)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![name, description, notes, email, contract_id, date_range],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[allow(clippy::too_many_arguments)]
pub fn update_email_template(
    conn: &Connection,
    id: i64,
    name: &str,
    description: Option<&str>,
    notes: Option<&str>,
    email: Option<&str>,
    contract_id: Option<i64>,
    date_range: &str,
) -> DomainResult<()> {
    conn.execute(
        "UPDATE email_templates SET name = ?1, description = ?2, notes = ?3, email = ?4,
            contract_id = ?5, date_range = ?6
         WHERE id = ?7",
        params![name, description, notes, email, contract_id, date_range, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_email_template(conn: &Connection, id: i64) -> DomainResult<()> {
    let deleted = conn.execute("DELETE FROM email_templates WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
    if deleted == 0 {
        return Err(format!("email template {id} not found"));
    }
    Ok(())
}

/// Active contracts with zero time entries in [from, to], optionally narrowed to one
/// contract. Deliberately not shared with `reports::contracts_without_time` — that
/// function lives on an independent, still-unmerged branch; once both land, one should
/// be folded into the other.
fn idle_contracts(conn: &Connection, from: &str, to: &str, only_contract_id: Option<i64>) -> DomainResult<Vec<IdleContract>> {
    let sql = "SELECT c.id, c.name, cl.id, cl.name
         FROM contracts c
         JOIN clients cl ON cl.id = c.client_id
         WHERE c.archived_at IS NULL
           AND (?3 IS NULL OR c.id = ?3)
           AND c.id NOT IN (
             SELECT DISTINCT te.contract_id FROM time_entries te
             WHERE te.deleted_at IS NULL AND te.started_at >= ?1 AND te.started_at <= ?2
           )
         ORDER BY cl.name, c.name";
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![from, to, only_contract_id], |row| {
            Ok(IdleContract {
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

fn format_body(notes: Option<&str>, date_label: &str, contracts: &[IdleContract]) -> String {
    let mut body = String::new();
    if let Some(notes) = notes {
        let trimmed = notes.trim();
        if !trimmed.is_empty() {
            body.push_str(trimmed);
            body.push_str("\n\n");
        }
    }
    if contracts.is_empty() {
        body.push_str(&format!("No contracts without logged time for {date_label}."));
    } else {
        body.push_str(&format!("The following contracts have no time logged for {date_label}:\n\n"));
        for c in contracts {
            body.push_str(&format!("- {} — {}\n", c.client_name, c.contract_name));
        }
    }
    body
}

/// Builds the subject/body for a template's "no time logged" email as of `reference_date`
/// (YYYY-MM-DD). The app never sends this itself — the caller copies it or hands it to
/// the user's own mail client. `utc_offset_minutes` is the caller's offset east of UTC,
/// so the range covers local rather than UTC days.
pub fn generate_no_time_email(
    conn: &Connection,
    template_id: i64,
    reference_date: &str,
    utc_offset_minutes: i32,
) -> DomainResult<NoTimeEmailPreview> {
    let template = get_email_template(conn, template_id)?;
    let reference: NaiveDate = reference_date.parse().map_err(|e| format!("invalid date: {e}"))?;
    let (start, end) = resolve_date_range_preset(&template.date_range, reference)?;
    let (from, to) = to_rfc3339_bounds(start, end, utc_offset_minutes);

    let contracts = idle_contracts(conn, &from, &to, template.contract_id)?;
    let date_label = format!("{} to {}", start.format("%Y-%m-%d"), end.format("%Y-%m-%d"));

    Ok(NoTimeEmailPreview {
        subject: format!("{} — {}", template.name, date_label),
        body: format_body(template.notes.as_deref(), &date_label, &contracts),
        to: template.email.clone(),
        from: start.format("%Y-%m-%d").to_string(),
        to_date: end.format("%Y-%m-%d").to_string(),
        contracts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::contracts::{create_client, create_contract};
    use crate::domain::time_entries::create_manual_entry;
    use std::fs;

    fn test_db(name: &str) -> Connection {
        let pid = std::process::id();
        let db_path = std::env::temp_dir().join(format!("timetracker_test_email_templates_{pid}_{name}.sqlite"));
        let _ = fs::remove_file(&db_path);
        crate::db::open(&db_path).expect("open db")
    }

    #[test]
    fn generates_a_list_of_idle_contracts_for_the_resolved_period() {
        let conn = test_db("idle_list");
        let client_id = create_client(&conn, "Client", None).expect("create client");
        let worked = create_contract(&conn, client_id, "Worked", "USD", 50.0).expect("create contract");
        let idle = create_contract(&conn, client_id, "Idle", "USD", 50.0).expect("create contract");

        // "2026-08-24" is a Monday; the previous week is 2026-08-17..2026-08-23.
        create_manual_entry(&conn, worked, "2026-08-18T09:00:00Z", "2026-08-18T10:00:00Z", None, None)
            .expect("create entry");

        let template_id = create_email_template(&conn, "Weekly check", None, None, Some("me@example.com"), None, "previous_week")
            .expect("create template");

        let preview = generate_no_time_email(&conn, template_id, "2026-08-24", 0).expect("generate email");

        assert_eq!(preview.to.as_deref(), Some("me@example.com"));
        assert_eq!(preview.contracts.len(), 1);
        assert_eq!(preview.contracts[0].contract_id, idle);
        assert!(preview.body.contains("Idle"));
        assert!(!preview.body.contains("Worked"));
    }

    #[test]
    fn scoping_to_one_contract_ignores_other_idle_contracts() {
        let conn = test_db("scoped");
        let client_id = create_client(&conn, "Client", None).expect("create client");
        let target = create_contract(&conn, client_id, "Target", "USD", 50.0).expect("create contract");
        create_contract(&conn, client_id, "Other", "USD", 50.0).expect("create contract");

        let template_id =
            create_email_template(&conn, "Just target", None, None, None, Some(target), "previous_week").expect("create template");

        let preview = generate_no_time_email(&conn, template_id, "2026-08-24", 0).expect("generate email");

        assert_eq!(preview.contracts.len(), 1);
        assert_eq!(preview.contracts[0].contract_id, target);
    }

    #[test]
    fn an_empty_idle_list_still_produces_a_readable_body() {
        let conn = test_db("empty");
        let client_id = create_client(&conn, "Client", None).expect("create client");
        let contract_id = create_contract(&conn, client_id, "Contract", "USD", 50.0).expect("create contract");
        create_manual_entry(&conn, contract_id, "2026-08-18T09:00:00Z", "2026-08-18T10:00:00Z", None, None)
            .expect("create entry");

        let template_id =
            create_email_template(&conn, "All clear", None, None, None, None, "previous_week").expect("create template");

        let preview = generate_no_time_email(&conn, template_id, "2026-08-24", 0).expect("generate email");

        assert!(preview.contracts.is_empty());
        assert!(preview.body.contains("No contracts without logged time"));
    }

    #[test]
    fn notes_are_prepended_to_the_generated_body() {
        let conn = test_db("notes");
        let client_id = create_client(&conn, "Client", None).expect("create client");
        create_contract(&conn, client_id, "Contract", "USD", 50.0).expect("create contract");

        let template_id = create_email_template(
            &conn,
            "With notes",
            None,
            Some("Please follow up with these clients."),
            None,
            None,
            "previous_week",
        )
        .expect("create template");

        let preview = generate_no_time_email(&conn, template_id, "2026-08-24", 0).expect("generate email");

        assert!(preview.body.starts_with("Please follow up with these clients."));
    }
}
