use chrono::Local;
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
    (
        "0015_contract_hour_adjustments",
        include_str!("../../migrations/0015_contract_hour_adjustments.sql"),
    ),
    (
        "0016_email_templates",
        include_str!("../../migrations/0016_email_templates.sql"),
    ),
    (
        "0017_email_template_content",
        include_str!("../../migrations/0017_email_template_content.sql"),
    ),
    (
        "0018_pay_week_start",
        include_str!("../../migrations/0018_pay_week_start.sql"),
    ),
    (
        "0019_entry_local_date",
        include_str!("../../migrations/0019_entry_local_date.sql"),
    ),
];

pub fn open(db_path: &Path) -> Result<Connection, Box<dyn std::error::Error>> {
    if db_path.exists() {
        backup_before_upgrade(db_path)?;
    }

    let conn = Connection::open(db_path)?;
    conn.pragma_update(None, "foreign_keys", true)?;
    apply_migrations(&conn)?;
    Ok(conn)
}

/// Copies the raw database file into a `backups/` folder alongside it before an app
/// upgrade applies new migrations — a schema change that goes wrong should never cost
/// the user their data. Only runs when there's an actual migration gap to close (an
/// existing, non-empty `schema_version` short of `MIGRATIONS.len()`); a brand-new
/// database or one already fully migrated has nothing worth backing up.
fn backup_before_upgrade(db_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let applied: i64 = {
        let conn = Connection::open(db_path)?;
        let table_exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'schema_version'",
            [],
            |row| row.get(0),
        )?;
        if table_exists == 0 {
            0
        } else {
            conn.query_row("SELECT COUNT(*) FROM schema_version", [], |row| row.get(0))?
        }
    };

    if applied > 0 && (applied as usize) < MIGRATIONS.len() {
        let backup_dir = db_path.parent().ok_or("database path has no parent directory")?.join("backups");
        std::fs::create_dir_all(&backup_dir)?;
        let file_name = db_path.file_name().ok_or("database path has no file name")?.to_string_lossy();
        let stamp = Local::now().format("%Y-%m-%d_%H%M%S");
        let backup_path = backup_dir.join(format!("{stamp}_pre-upgrade_{file_name}"));
        std::fs::copy(db_path, &backup_path)?;
    }

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("timetracker_test_db_mod_{}_{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    #[test]
    fn opening_a_brand_new_database_does_not_create_a_backup() {
        let dir = test_dir("fresh");
        let db_path = dir.join("timetracker.sqlite");

        open(&db_path).expect("open fresh db");

        assert!(!dir.join("backups").exists(), "a brand-new database has nothing worth backing up");
    }

    #[test]
    fn reopening_an_up_to_date_database_does_not_create_a_backup() {
        let dir = test_dir("up_to_date");
        let db_path = dir.join("timetracker.sqlite");

        open(&db_path).expect("open and fully migrate");
        drop(open(&db_path).expect("reopen already-current db"));

        assert!(!dir.join("backups").exists(), "no pending migrations means no upgrade, so no backup");
    }

    /// The 0019 backfill has to reach entries that already existed, and has to agree
    /// with what the write path would have recorded for the same instant. A row left
    /// with a NULL day would silently vanish from every period query.
    #[test]
    fn the_local_date_migration_backfills_entries_written_before_it() {
        use chrono::{DateTime, Local, Offset};

        let dir = test_dir("local_date_backfill");
        let db_path = dir.join("timetracker.sqlite");

        // An older install: migrated up to 0018, so time_entries exists but has neither
        // of the columns 0019 adds.
        let legacy_count = MIGRATIONS.iter().position(|(name, _)| *name == "0019_entry_local_date").expect("0019 present");
        {
            let conn = Connection::open(&db_path).expect("create db file");
            conn.pragma_update(None, "foreign_keys", true).expect("enable fks");
            conn.execute_batch(
                "CREATE TABLE schema_version (
                    version    INTEGER PRIMARY KEY,
                    name       TEXT NOT NULL,
                    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
                )",
            )
            .expect("create schema_version");
            for (index, (name, sql)) in MIGRATIONS.iter().take(legacy_count).enumerate() {
                conn.execute_batch(sql).unwrap_or_else(|e| panic!("apply {name}: {e}"));
                conn.execute("INSERT INTO schema_version (version, name) VALUES (?1, ?2)", ((index + 1) as i64, *name))
                    .expect("record migration");
            }

            conn.execute("INSERT INTO clients (name) VALUES ('Legacy Client')", []).expect("client");
            conn.execute("INSERT INTO contracts (client_id, name, currency) VALUES (1, 'Legacy', 'USD')", [])
                .expect("contract");
            // Just after UTC midnight: the case where the UTC day and the local day differ
            // west of Greenwich, which is exactly what this column exists to pin down.
            conn.execute(
                "INSERT INTO time_entries (contract_id, started_at, ended_at, duration_secs, rate_snapshot, source)
                 VALUES (1, '2026-08-24T00:30:00Z', '2026-08-24T01:30:00Z', 3600, 50.0, 'manual')",
                [],
            )
            .expect("legacy entry");
        }

        let conn = open(&db_path).expect("migrate the rest of the way");

        let (local_date, offset): (String, i32) = conn
            .query_row("SELECT local_date, utc_offset_minutes FROM time_entries WHERE id = 1", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .expect("backfilled row");

        let expected = DateTime::parse_from_rfc3339("2026-08-24T00:30:00Z").unwrap().with_timezone(&Local);
        assert_eq!(local_date, expected.date_naive().format("%Y-%m-%d").to_string());
        assert_eq!(offset, expected.offset().fix().local_minus_utc() / 60);
    }

    /// Re-running the whole chain on a database that already has 0019 applied must be a
    /// no-op — the runner tracks by count, and a second ALTER TABLE would fail outright.
    #[test]
    fn reopening_after_the_local_date_migration_changes_nothing() {
        let dir = test_dir("local_date_idempotent");
        let db_path = dir.join("timetracker.sqlite");

        open(&db_path).expect("first open");
        let conn = open(&db_path).expect("second open must not reapply 0019");

        let applied: i64 = conn.query_row("SELECT COUNT(*) FROM schema_version", [], |row| row.get(0)).expect("count");
        assert_eq!(applied as usize, MIGRATIONS.len());
    }

    #[test]
    fn opening_an_existing_database_with_pending_migrations_backs_it_up_first() {
        let dir = test_dir("upgrade");
        let db_path = dir.join("timetracker.sqlite");

        // Simulate an older install: a real db with only the first 2 migrations applied,
        // short of the full MIGRATIONS list this binary knows about.
        {
            let conn = Connection::open(&db_path).expect("create db file");
            conn.execute_batch(MIGRATIONS[0].1).expect("apply migration 0");
            conn.execute_batch(MIGRATIONS[1].1).expect("apply migration 1");
            conn.execute_batch(
                "CREATE TABLE schema_version (
                    version    INTEGER PRIMARY KEY,
                    name       TEXT NOT NULL,
                    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
                )",
            )
            .expect("create schema_version");
            conn.execute(
                "INSERT INTO schema_version (version, name) VALUES (?1, ?2)",
                (1i64, MIGRATIONS[0].0),
            )
            .expect("record migration 0");
            conn.execute(
                "INSERT INTO schema_version (version, name) VALUES (?1, ?2)",
                (2i64, MIGRATIONS[1].0),
            )
            .expect("record migration 1");
        }

        open(&db_path).expect("open and migrate the rest of the way");

        let backup_dir = dir.join("backups");
        let backups: Vec<_> = std::fs::read_dir(&backup_dir).expect("read backups dir").collect::<Result<_, _>>().unwrap();
        assert_eq!(backups.len(), 1, "expected exactly one pre-upgrade backup");

        let backup_size = backups[0].metadata().unwrap().len();
        assert!(backup_size > 0, "backup should be a real copy of the pre-upgrade database, not empty");
    }
}
