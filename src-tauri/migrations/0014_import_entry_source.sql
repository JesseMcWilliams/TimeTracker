-- Bulk-imported time entries should be distinguishable from ordinary manual entries
-- (e.g. "Source" column on the Entries page), so add 'import' as an allowed value.
-- SQLite can't ALTER a CHECK constraint directly, so the table is rebuilt: create a
-- copy with the new constraint, copy every row across (ids preserved), drop the old
-- table, rename the copy into place, and recreate its indexes. foreign_keys is
-- toggled off around the rebuild per SQLite's documented procedure for this kind of
-- schema change (PRAGMA foreign_keys has no effect inside a transaction, so it must be
-- set outside one) — time_entries is the target of time_entry_tags' foreign key, and
-- the toggle avoids that briefly-dangling reference tripping enforcement mid-rebuild.
PRAGMA foreign_keys = OFF;

CREATE TABLE time_entries_new (
    id                INTEGER PRIMARY KEY,
    contract_id       INTEGER NOT NULL REFERENCES contracts(id),
    started_at        TEXT NOT NULL,
    ended_at          TEXT,
    duration_secs     INTEGER,
    rate_snapshot     REAL NOT NULL,
    notes             TEXT,
    source            TEXT NOT NULL CHECK (source IN ('manual', 'timer', 'obsidian_import', 'import')),
    external_ref      TEXT,
    created_at        TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at        TEXT NOT NULL DEFAULT (datetime('now')),
    deleted_at        TEXT,
    tracking_code_id  INTEGER REFERENCES tracking_codes(id)
);

INSERT INTO time_entries_new
    (id, contract_id, started_at, ended_at, duration_secs, rate_snapshot, notes,
     source, external_ref, created_at, updated_at, deleted_at, tracking_code_id)
SELECT id, contract_id, started_at, ended_at, duration_secs, rate_snapshot, notes,
       source, external_ref, created_at, updated_at, deleted_at, tracking_code_id
FROM time_entries;

DROP TABLE time_entries;
ALTER TABLE time_entries_new RENAME TO time_entries;

CREATE INDEX idx_time_entries_contract_id ON time_entries(contract_id);
CREATE INDEX idx_time_entries_active ON time_entries(ended_at) WHERE ended_at IS NULL;

PRAGMA foreign_keys = ON;
