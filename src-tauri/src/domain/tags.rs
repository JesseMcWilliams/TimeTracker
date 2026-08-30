use rusqlite::{params, Connection};
use serde::Serialize;

use super::DomainResult;

#[derive(Debug, Serialize)]
pub struct Tag {
    pub id: i64,
    pub name: String,
}

pub fn list_tags(conn: &Connection) -> DomainResult<Vec<Tag>> {
    let mut stmt = conn
        .prepare("SELECT id, name FROM tags ORDER BY name")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn get_or_create_tag(conn: &Connection, name: &str) -> DomainResult<i64> {
    conn.execute(
        "INSERT INTO tags (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
        params![name],
    )
    .map_err(|e| e.to_string())?;
    conn.query_row("SELECT id FROM tags WHERE name = ?1", params![name], |row| row.get(0))
        .map_err(|e| e.to_string())
}

pub fn add_tag_to_entry(conn: &Connection, time_entry_id: i64, tag_name: &str) -> DomainResult<()> {
    let tag_id = get_or_create_tag(conn, tag_name)?;
    conn.execute(
        "INSERT INTO time_entry_tags (time_entry_id, tag_id) VALUES (?1, ?2)
         ON CONFLICT(time_entry_id, tag_id) DO NOTHING",
        params![time_entry_id, tag_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn remove_tag_from_entry(conn: &Connection, time_entry_id: i64, tag_name: &str) -> DomainResult<()> {
    conn.execute(
        "DELETE FROM time_entry_tags
         WHERE time_entry_id = ?1 AND tag_id = (SELECT id FROM tags WHERE name = ?2)",
        params![time_entry_id, tag_name],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn tags_for_entry(conn: &Connection, time_entry_id: i64) -> DomainResult<Vec<String>> {
    let mut stmt = conn
        .prepare(
            "SELECT t.name FROM tags t
             JOIN time_entry_tags tet ON tet.tag_id = t.id
             WHERE tet.time_entry_id = ?1
             ORDER BY t.name",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![time_entry_id], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}
