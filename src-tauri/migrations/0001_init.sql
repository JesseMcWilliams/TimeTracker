CREATE TABLE clients (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    notes       TEXT,
    archived_at TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE contracts (
    id          INTEGER PRIMARY KEY,
    client_id   INTEGER NOT NULL REFERENCES clients(id),
    name        TEXT NOT NULL,
    currency    TEXT NOT NULL DEFAULT 'USD',
    archived_at TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Rate HISTORY, not a single column — new entries snapshot the currently
-- effective rate into time_entries.rate_snapshot and never re-read this later.
CREATE TABLE contract_rates (
    id             INTEGER PRIMARY KEY,
    contract_id    INTEGER NOT NULL REFERENCES contracts(id),
    hourly_rate    REAL NOT NULL,
    effective_from TEXT NOT NULL,
    created_at     TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE time_entries (
    id            INTEGER PRIMARY KEY,
    contract_id   INTEGER NOT NULL REFERENCES contracts(id),
    started_at    TEXT NOT NULL,      -- ISO 8601
    ended_at      TEXT,               -- NULL while a timer is running
    duration_secs INTEGER,            -- denormalized on stop/edit for fast reporting
    rate_snapshot REAL NOT NULL,      -- immutable after creation
    notes         TEXT,
    source        TEXT NOT NULL CHECK (source IN ('manual','timer','obsidian_import')),
    external_ref  TEXT,               -- source file path/anchor for obsidian re-import dedup
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE tags (
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE time_entry_tags (
    time_entry_id INTEGER NOT NULL REFERENCES time_entries(id) ON DELETE CASCADE,
    tag_id        INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (time_entry_id, tag_id)
);

CREATE INDEX idx_contracts_client_id ON contracts(client_id);
CREATE INDEX idx_contract_rates_contract_id ON contract_rates(contract_id);
CREATE INDEX idx_time_entries_contract_id ON time_entries(contract_id);
CREATE INDEX idx_time_entries_active ON time_entries(ended_at) WHERE ended_at IS NULL;
