use rusqlite::Connection;
use std::path::Path;

/// Embedded migrations, applied in order. Add new entries here as the schema evolves —
/// each one runs exactly once, tracked by `schema_version`.
const MIGRATIONS: &[(&str, &str)] = &[
    ("0001_init", include_str!("../../migrations/0001_init.sql")),
    (
        "0002_soft_delete",
        include_str!("../../migrations/0002_soft_delete.sql"),
    ),
    (
        "0003_tracking_codes",
        include_str!("../../migrations/0003_tracking_codes.sql"),
    ),
    (
        "0004_client_prime_sub",
        include_str!("../../migrations/0004_client_prime_sub.sql"),
    ),
    (
        "0005_user_profile_and_metadata",
        include_str!("../../migrations/0005_user_profile_and_metadata.sql"),
    ),
    (
        "0006_window_defaults_and_quick_timer",
        include_str!("../../migrations/0006_window_defaults_and_quick_timer.sql"),
    ),
    (
        "0007_launch_position",
        include_str!("../../migrations/0007_launch_position.sql"),
    ),
    (
        "0008_custom_position_and_min_increment",
        include_str!("../../migrations/0008_custom_position_and_min_increment.sql"),
    ),
    (
        "0009_contract_notes",
        include_str!("../../migrations/0009_contract_notes.sql"),
    ),
    ("0010_theme", include_str!("../../migrations/0010_theme.sql")),
    (
        "0011_contract_filename_date",
        include_str!("../../migrations/0011_contract_filename_date.sql"),
    ),
    (
        "0012_import_templates",
        include_str!("../../migrations/0012_import_templates.sql"),
    ),
    (
        "0013_import_template_column_aliases",
        include_str!("../../migrations/0013_import_template_column_aliases.sql"),
    ),
    (
        "0014_import_entry_source",
        include_str!("../../migrations/0014_import_entry_source.sql"),
    ),
];

pub fn open(db_path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.pragma_update(None, "foreign_keys", true)?;
    apply_migrations(&conn)?;
    Ok(conn)
}

fn apply_migrations(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version    INTEGER PRIMARY KEY,
            name       TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )?;

    let applied: i64 = conn.query_row("SELECT COUNT(*) FROM schema_version", [], |row| row.get(0))?;
    let applied = applied as usize;

    for (i, (name, sql)) in MIGRATIONS.iter().enumerate().skip(applied) {
        conn.execute_batch(sql)?;
        conn.execute(
            "INSERT INTO schema_version (version, name) VALUES (?1, ?2)",
            (i as i64 + 1, name),
        )?;
    }

    Ok(())
}
