-- Tracks a contract's total budgeted hours as an append-only log of adjustments
-- (positive to add hours, negative to remove), mirroring how contract_rates tracks
-- rate changes over time. A contract's total budgeted hours is the sum of its
-- adjustments; a contract with no rows has no hours budget set.
CREATE TABLE contract_hour_adjustments (
    id          INTEGER PRIMARY KEY,
    contract_id INTEGER NOT NULL REFERENCES contracts(id),
    hours_delta REAL NOT NULL,
    note        TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_contract_hour_adjustments_contract_id ON contract_hour_adjustments(contract_id);
