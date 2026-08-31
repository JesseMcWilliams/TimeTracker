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
    pub date_column: String,
    pub start_column: String,
    pub end_column: String,
    pub category_column: Option<String>,
    pub notes_column: Option<String>,
}

const COLUMNS: &str = "id, name, notes, is_default, date_column, start_column, end_column, \
     category_column, notes_column";

fn row_to_template(row: &rusqlite::Row) -> rusqlite::Result<ImportTemplate> {
    Ok(ImportTemplate {
        id: row.get(0)?,
        name: row.get(1)?,
        notes: row.get(2)?,
        is_default: row.get::<_, i64>(3)? != 0,
        date_column: row.get(4)?,
        start_column: row.get(5)?,
        end_column: row.get(6)?,
        category_column: row.get(7)?,
        notes_column: row.get(8)?,
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
    date_column: &str,
    start_column: &str,
    end_column: &str,
    category_column: Option<&str>,
    notes_column: Option<&str>,
) -> DomainResult<i64> {
    conn.execute(
        "INSERT INTO import_templates
            (name, notes, is_default, date_column, start_column, end_column, category_column, notes_column)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![name, notes, is_default as i64, date_column, start_column, end_column, category_column, notes_column],
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
    date_column: &str,
    start_column: &str,
    end_column: &str,
    category_column: Option<&str>,
    notes_column: Option<&str>,
) -> DomainResult<()> {
    conn.execute(
        "UPDATE import_templates SET name = ?1, notes = ?2, is_default = ?3, date_column = ?4,
            start_column = ?5, end_column = ?6, category_column = ?7, notes_column = ?8
         WHERE id = ?9",
        params![
            name,
            notes,
            is_default as i64,
            date_column,
            start_column,
            end_column,
            category_column,
            notes_column,
            id
        ],
    )
    .map_err(|e| e.to_string())?;
    if is_default {
        clear_other_defaults(conn, Some(id))?;
    }
    Ok(())
}
