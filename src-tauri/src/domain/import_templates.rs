use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::DomainResult;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportTemplate {
    pub id: i64,
    pub name: String,
    pub notes: Option<String>,
    pub is_default: bool,
    pub date_columns: String,
    pub start_columns: String,
    pub end_columns: String,
    pub category_columns: Option<String>,
    pub notes_columns: Option<String>,
}

/// Splits a template field's comma-separated alias list (e.g. "Start, Start Time")
/// into trimmed, non-empty names, tried in order against a file's header row.
pub fn split_aliases(s: &str) -> Vec<String> {
    s.split(',').map(|part| part.trim().to_string()).filter(|part| !part.is_empty()).collect()
}

const COLUMNS: &str = "id, name, notes, is_default, date_columns, start_columns, end_columns, \
     category_columns, notes_columns";

fn row_to_template(row: &rusqlite::Row) -> rusqlite::Result<ImportTemplate> {
    Ok(ImportTemplate {
        id: row.get(0)?,
        name: row.get(1)?,
        notes: row.get(2)?,
        is_default: row.get::<_, i64>(3)? != 0,
        date_columns: row.get(4)?,
        start_columns: row.get(5)?,
        end_columns: row.get(6)?,
        category_columns: row.get(7)?,
        notes_columns: row.get(8)?,
    })
}

pub fn list_import_templates(conn: &Connection) -> DomainResult<Vec<ImportTemplate>> {
    let mut stmt = conn
        .prepare(&format!("SELECT {COLUMNS} FROM import_templates ORDER BY name"))
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], row_to_template).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn get_import_template(conn: &Connection, id: i64) -> DomainResult<ImportTemplate> {
    conn.query_row(&format!("SELECT {COLUMNS} FROM import_templates WHERE id = ?1"), params![id], row_to_template)
        .map_err(|e| format!("import template {id} not found: {e}"))
}

/// If `is_default` is set, clears the flag on every other template first — exactly one
/// template is ever the default at a time, enforced here rather than in the schema.
fn clear_other_defaults(conn: &Connection, keep_id: Option<i64>) -> DomainResult<()> {
    conn.execute(
        "UPDATE import_templates SET is_default = 0 WHERE id IS NOT ?1",
        params![keep_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn create_import_template(
    conn: &Connection,
    name: &str,
    notes: Option<&str>,
    is_default: bool,
    date_columns: &str,
    start_columns: &str,
    end_columns: &str,
    category_columns: Option<&str>,
    notes_columns: Option<&str>,
) -> DomainResult<i64> {
    conn.execute(
        "INSERT INTO import_templates
            (name, notes, is_default, date_columns, start_columns, end_columns, category_columns, notes_columns)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            name,
            notes,
            is_default as i64,
            date_columns,
            start_columns,
            end_columns,
            category_columns,
            notes_columns
        ],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    if is_default {
        clear_other_defaults(conn, Some(id))?;
    }
    Ok(id)
}

#[allow(clippy::too_many_arguments)]
pub fn update_import_template(
    conn: &Connection,
    id: i64,
    name: &str,
    notes: Option<&str>,
    is_default: bool,
    date_columns: &str,
    start_columns: &str,
    end_columns: &str,
    category_columns: Option<&str>,
    notes_columns: Option<&str>,
) -> DomainResult<()> {
    conn.execute(
        "UPDATE import_templates SET name = ?1, notes = ?2, is_default = ?3, date_columns = ?4,
            start_columns = ?5, end_columns = ?6, category_columns = ?7, notes_columns = ?8
         WHERE id = ?9",
        params![
            name,
            notes,
            is_default as i64,
            date_columns,
            start_columns,
            end_columns,
            category_columns,
            notes_columns,
            id
        ],
    )
    .map_err(|e| e.to_string())?;
    if is_default {
        clear_other_defaults(conn, Some(id))?;
    }
    Ok(())
}

/// Import always needs at least one template to fall back to, so deleting the last
/// remaining one is blocked rather than leaving the Import page with nothing to select.
pub fn delete_import_template(conn: &Connection, id: i64) -> DomainResult<()> {
    let count: i64 =
        conn.query_row("SELECT COUNT(*) FROM import_templates", [], |row| row.get(0)).map_err(|e| e.to_string())?;
    if count <= 1 {
        return Err("cannot delete the last remaining import template — at least one must exist".to_string());
    }
    let deleted = conn.execute("DELETE FROM import_templates WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
    if deleted == 0 {
        return Err(format!("import template {id} not found"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_conn() -> Connection {
        let path = std::env::temp_dir()
            .join(format!("timetracker_test_{}_import_templates.sqlite", std::process::id()));
        let _ = std::fs::remove_file(&path);
        crate::db::open(&path).expect("open db")
    }

    #[test]
    fn deleting_the_last_remaining_template_is_blocked() {
        let conn = temp_conn();
        // Migration 0012 seeds two templates ("Default", "Activity Sheet") — delete one
        // to get down to exactly one, which should then refuse further deletion.
        let templates = list_import_templates(&conn).expect("list templates");
        assert_eq!(templates.len(), 2);

        delete_import_template(&conn, templates[0].id).expect("delete first template");
        let remaining = list_import_templates(&conn).expect("list templates");
        assert_eq!(remaining.len(), 1);

        let err = delete_import_template(&conn, remaining[0].id).expect_err("must block deleting the last one");
        assert!(err.contains("last remaining"), "unexpected message: {err}");
        assert_eq!(list_import_templates(&conn).expect("list templates").len(), 1);
    }
}
