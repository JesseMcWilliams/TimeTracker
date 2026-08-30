-- Tracking codes belong to a client (not a specific contract), since some clients
-- require every time entry to be tagged with one of their own codes.
CREATE TABLE tracking_codes (
    id          INTEGER PRIMARY KEY,
    client_id   INTEGER NOT NULL REFERENCES clients(id),
    code        TEXT NOT NULL,
    description TEXT,
    archived_at TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (client_id, code)
);

CREATE INDEX idx_tracking_codes_client_id ON tracking_codes(client_id);

ALTER TABLE time_entries ADD COLUMN tracking_code_id INTEGER REFERENCES tracking_codes(id);
