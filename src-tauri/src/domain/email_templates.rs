use chrono::NaiveDate;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::period::{resolve_date_range_preset, to_local_date_bounds};
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
    /// Author-written subject line, with placeholders (see `expand_placeholders`).
    /// `None` falls back to the built-in wording, so templates predating this field
    /// keep generating the email they always did.
    pub subject_template: Option<String>,
    /// Author-written body. `None` falls back to the built-in wording.
    pub body_template: Option<String>,
    /// Exactly one template may carry this; the Reports page preselects it.
    pub is_default: bool,
    /// How `{StartDate}`/`{EndDate}` render: "iso", "us", or "long".
    pub date_format: String,
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

const COLUMNS: &str = "id, name, description, notes, email, contract_id, date_range,      subject_template, body_template, is_default, date_format";

fn row_to_template(row: &rusqlite::Row) -> rusqlite::Result<EmailTemplate> {
    Ok(EmailTemplate {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        notes: row.get(3)?,
        email: row.get(4)?,
        contract_id: row.get(5)?,
        date_range: row.get(6)?,
        subject_template: row.get(7)?,
        body_template: row.get(8)?,
        is_default: row.get::<_, i64>(9)? != 0,
        date_format: row.get(10)?,
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
    subject_template: Option<&str>,
    body_template: Option<&str>,
    date_format: &str,
) -> DomainResult<i64> {
    conn.execute(
        "INSERT INTO email_templates (name, description, notes, email, contract_id, date_range,
            subject_template, body_template, date_format)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![name, description, notes, email, contract_id, date_range, subject_template, body_template, date_format],
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
    subject_template: Option<&str>,
    body_template: Option<&str>,
    date_format: &str,
) -> DomainResult<()> {
    conn.execute(
        "UPDATE email_templates SET name = ?1, description = ?2, notes = ?3, email = ?4,
            contract_id = ?5, date_range = ?6, subject_template = ?7, body_template = ?8,
            date_format = ?9
         WHERE id = ?10",
        params![
            name,
            description,
            notes,
            email,
            contract_id,
            date_range,
            subject_template,
            body_template,
            date_format,
            id
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Makes `id` the default template, clearing whichever template held it before.
/// Both writes go in one transaction because a partial unique index forbids two
/// defaults existing even momentarily.
pub fn set_default_email_template(conn: &Connection, id: i64) -> DomainResult<()> {
    let exists: i64 = conn
        .query_row("SELECT COUNT(*) FROM email_templates WHERE id = ?1", params![id], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    if exists == 0 {
        return Err(format!("email template {id} not found"));
    }

    conn.execute("BEGIN", []).map_err(|e| e.to_string())?;
    let result = (|| {
        conn.execute("UPDATE email_templates SET is_default = 0 WHERE is_default = 1", [])?;
        conn.execute("UPDATE email_templates SET is_default = 1 WHERE id = ?1", params![id])?;
        Ok::<(), rusqlite::Error>(())
    })();
    match result {
        Ok(()) => {
            conn.execute("COMMIT", []).map_err(|e| e.to_string())?;
            Ok(())
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK", []);
            Err(e.to_string())
        }
    }
}

/// Clears the default flag entirely, leaving no template preselected.
pub fn clear_default_email_template(conn: &Connection) -> DomainResult<()> {
    conn.execute("UPDATE email_templates SET is_default = 0 WHERE is_default = 1", [])
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
             WHERE te.deleted_at IS NULL AND te.local_date >= ?1 AND te.local_date <= ?2
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

/// Renders a date the way a template asks for it. Unknown formats fall back to ISO
/// rather than erroring — a template is user data, and a bad value should not block
/// generating the email.
fn format_date(date: NaiveDate, date_format: &str) -> String {
    match date_format {
        "us" => date.format("%m/%d/%Y").to_string(),
        "long" => date.format("%b %-d, %Y").to_string(),
        _ => date.format("%Y-%m-%d").to_string(),
    }
}

/// The placeholders a subject or body template may use. Matching is case-insensitive
/// on the name inside the braces, so `{startdate}` and `{StartDate}` both work — the
/// documented spelling is the CamelCase one shown in the editor's hint text.
///
/// No Rust caller reads this list — the editor's hint list lives in `api.ts` — but it
/// belongs beside the expander that implements it, and a test keeps it honest by
/// checking every name here actually substitutes.
#[allow(dead_code)]
pub const PLACEHOLDERS: &[&str] = &["StartDate", "EndDate", "DateRange", "Contracts", "ContractCount", "TemplateName"];

/// Substitutes `{Placeholder}` tokens. Anything that is not a recognised placeholder is
/// left exactly as typed, so a stray brace in prose survives instead of vanishing.
fn expand_placeholders(template: &str, values: &[(&str, String)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('}') else {
            out.push_str(&rest[open..]);
            return out;
        };
        let name = &after_open[..close];
        match values.iter().find(|(key, _)| key.eq_ignore_ascii_case(name)) {
            Some((_, value)) => out.push_str(value),
            None => {
                out.push('{');
                out.push_str(name);
                out.push('}');
            }
        }
        rest = &after_open[close + 1..];
    }

    out.push_str(rest);
    out
}

/// The bulleted "client — contract" list that `{Contracts}` expands to.
fn contract_list(contracts: &[IdleContract]) -> String {
    contracts
        .iter()
        .map(|c| format!("- {} — {}", c.client_name, c.contract_name))
        .collect::<Vec<_>>()
        .join("
")
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
/// the user's own mail client.
pub fn generate_no_time_email(
    conn: &Connection,
    template_id: i64,
    reference_date: &str,
) -> DomainResult<NoTimeEmailPreview> {
    let template = get_email_template(conn, template_id)?;
    let reference: NaiveDate = reference_date.parse().map_err(|e| format!("invalid date: {e}"))?;
    let (start, end) = resolve_date_range_preset(&template.date_range, reference)?;
    let (from, to) = to_local_date_bounds(start, end);

    let contracts = idle_contracts(conn, &from, &to, template.contract_id)?;

    let start_label = format_date(start, &template.date_format);
    let end_label = format_date(end, &template.date_format);
    let date_label = format!("{start_label} to {end_label}");
    let values = vec![
        ("StartDate", start_label.clone()),
        ("EndDate", end_label.clone()),
        ("DateRange", date_label.clone()),
        ("Contracts", contract_list(&contracts)),
        ("ContractCount", contracts.len().to_string()),
        ("TemplateName", template.name.clone()),
    ];

    // A template with no subject/body of its own keeps the built-in wording, so
    // upgrading does not quietly change the email an existing template produces.
    let subject = match template.subject_template.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(subject_template) => expand_placeholders(subject_template, &values),
        None => format!("{} — {}", template.name, date_label),
    };
    let body = match template.body_template.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(body_template) => expand_placeholders(body_template, &values),
        None => format_body(template.notes.as_deref(), &date_label, &contracts),
    };

    Ok(NoTimeEmailPreview {
        subject,
        body,
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

    /// A template that predates custom subject/body text (both NULL) must still produce
    /// the exact wording it always did — the upgrade path for existing templates.
    #[test]
    fn a_template_without_custom_text_keeps_the_built_in_wording() {
        let conn = test_db("builtin_wording");
        let client_id = create_client(&conn, "Client", None).expect("create client");
        create_contract(&conn, client_id, "Idle", "USD", 50.0).expect("create contract");

        let template_id =
            create_email_template(&conn, "Weekly check", None, None, None, None, "previous_week", None, None, "iso")
                .expect("create template");

        let preview = generate_no_time_email(&conn, template_id, "2026-08-24").expect("generate email");

        assert_eq!(preview.subject, "Weekly check — 2026-08-17 to 2026-08-23");
        assert!(preview.body.contains("The following contracts have no time logged"));
    }

    #[test]
    fn subject_and_body_placeholders_are_substituted() {
        let conn = test_db("placeholders");
        let client_id = create_client(&conn, "Acme", None).expect("create client");
        create_contract(&conn, client_id, "Support", "USD", 50.0).expect("create contract");

        let template_id = create_email_template(
            &conn,
            "Weekly",
            None,
            None,
            None,
            None,
            "previous_week",
            Some("Timesheets: {StartDate} to {EndDate}"),
            Some("{ContractCount} contract(s) idle for {DateRange}:\n{Contracts}"),
            "iso",
        )
        .expect("create template");

        let preview = generate_no_time_email(&conn, template_id, "2026-08-24").expect("generate email");

        assert_eq!(preview.subject, "Timesheets: 2026-08-17 to 2026-08-23");
        assert_eq!(preview.body, "1 contract(s) idle for 2026-08-17 to 2026-08-23:\n- Acme — Support");
    }

    #[test]
    fn the_date_format_choice_changes_how_placeholders_render() {
        let conn = test_db("date_format");
        let client_id = create_client(&conn, "Acme", None).expect("create client");
        create_contract(&conn, client_id, "Support", "USD", 50.0).expect("create contract");

        let us = create_email_template(
            &conn,
            "US",
            None,
            None,
            None,
            None,
            "previous_week",
            Some("{StartDate}..{EndDate}"),
            None,
            "us",
        )
        .expect("create us template");
        let long = create_email_template(
            &conn,
            "Long",
            None,
            None,
            None,
            None,
            "previous_week",
            Some("{StartDate}..{EndDate}"),
            None,
            "long",
        )
        .expect("create long template");

        assert_eq!(
            generate_no_time_email(&conn, us, "2026-08-24").expect("generate").subject,
            "08/17/2026..08/23/2026"
        );
        assert_eq!(
            generate_no_time_email(&conn, long, "2026-08-24").expect("generate").subject,
            "Aug 17, 2026..Aug 23, 2026"
        );
    }

    #[test]
    fn an_unrecognised_placeholder_is_left_as_typed() {
        let conn = test_db("unknown_placeholder");
        let client_id = create_client(&conn, "Acme", None).expect("create client");
        create_contract(&conn, client_id, "Support", "USD", 50.0).expect("create contract");

        let template_id = create_email_template(
            &conn,
            "Odd",
            None,
            None,
            None,
            None,
            "previous_week",
            Some("Budget {Nope} for {StartDate} — 50% {done"),
            None,
            "iso",
        )
        .expect("create template");

        let preview = generate_no_time_email(&conn, template_id, "2026-08-24").expect("generate email");

        assert_eq!(preview.subject, "Budget {Nope} for 2026-08-17 — 50% {done");
    }

    #[test]
    fn placeholder_names_match_regardless_of_case() {
        let conn = test_db("placeholder_case");
        let client_id = create_client(&conn, "Acme", None).expect("create client");
        create_contract(&conn, client_id, "Support", "USD", 50.0).expect("create contract");

        let template_id = create_email_template(
            &conn,
            "Case",
            None,
            None,
            None,
            None,
            "previous_week",
            Some("{startdate}/{ENDDATE}"),
            None,
            "iso",
        )
        .expect("create template");

        assert_eq!(
            generate_no_time_email(&conn, template_id, "2026-08-24").expect("generate").subject,
            "2026-08-17/2026-08-23"
        );
    }

    #[test]
    fn setting_a_default_clears_the_previous_one() {
        let conn = test_db("default_flag");
        let first = create_email_template(&conn, "First", None, None, None, None, "previous_week", None, None, "iso")
            .expect("create first");
        let second = create_email_template(&conn, "Second", None, None, None, None, "previous_week", None, None, "iso")
            .expect("create second");

        set_default_email_template(&conn, first).expect("set first default");
        set_default_email_template(&conn, second).expect("set second default");

        let templates = list_email_templates(&conn).expect("list");
        let defaults: Vec<i64> = templates.iter().filter(|t| t.is_default).map(|t| t.id).collect();
        assert_eq!(defaults, vec![second], "exactly one template stays default");
    }

    #[test]
    fn clearing_the_default_leaves_none_selected() {
        let conn = test_db("clear_default");
        let id = create_email_template(&conn, "Only", None, None, None, None, "previous_week", None, None, "iso")
            .expect("create template");

        set_default_email_template(&conn, id).expect("set default");
        clear_default_email_template(&conn).expect("clear default");

        assert!(list_email_templates(&conn).expect("list").iter().all(|t| !t.is_default));
    }

    /// Guards against PLACEHOLDERS listing a name the expander does not actually
    /// handle, which would advertise a placeholder that renders as literal text.
    #[test]
    fn every_documented_placeholder_is_actually_substituted() {
        let conn = test_db("documented_placeholders");
        let client_id = create_client(&conn, "Acme", None).expect("create client");
        create_contract(&conn, client_id, "Support", "USD", 50.0).expect("create contract");

        for placeholder in PLACEHOLDERS {
            let subject = format!("[{{{placeholder}}}]");
            let template_id = create_email_template(
                &conn,
                placeholder,
                None,
                None,
                None,
                None,
                "previous_week",
                Some(&subject),
                None,
                "iso",
            )
            .expect("create template");

            let preview = generate_no_time_email(&conn, template_id, "2026-08-24").expect("generate email");
            assert_ne!(preview.subject, subject, "{placeholder} was left as literal text");
        }
    }

    #[test]
    fn making_a_missing_template_the_default_is_an_error() {
        let conn = test_db("default_missing");
        assert!(set_default_email_template(&conn, 999).is_err());
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

        let template_id = create_email_template(
            &conn,
            "Weekly check",
            None,
            None,
            Some("me@example.com"),
            None,
            "previous_week",
            None,
            None,
            "iso",
        )
            .expect("create template");

        let preview = generate_no_time_email(&conn, template_id, "2026-08-24").expect("generate email");

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
            create_email_template(
            &conn,
            "Just target",
            None,
            None,
            None,
            Some(target),
            "previous_week",
            None,
            None,
            "iso",
        ).expect("create template");

        let preview = generate_no_time_email(&conn, template_id, "2026-08-24").expect("generate email");

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
            create_email_template(
            &conn,
            "All clear",
            None,
            None,
            None,
            None,
            "previous_week",
            None,
            None,
            "iso",
        ).expect("create template");

        let preview = generate_no_time_email(&conn, template_id, "2026-08-24").expect("generate email");

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
            None,
            None,
            "iso",
        )
        .expect("create template");

        let preview = generate_no_time_email(&conn, template_id, "2026-08-24").expect("generate email");

        assert!(preview.body.starts_with("Please follow up with these clients."));
    }
}
