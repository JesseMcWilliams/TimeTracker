-- Reusable templates for generating a "which contracts have no time logged" email —
-- the app never sends mail itself, it only builds the subject/body text for the user
-- to copy or hand off to their default mail client. contract_id NULL means "check all
-- active contracts"; a specific id scopes the check to just that one contract.
CREATE TABLE email_templates (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT,
    notes       TEXT,
    email       TEXT,
    contract_id INTEGER REFERENCES contracts(id),
    date_range  TEXT NOT NULL DEFAULT 'previous_week'
                CHECK (date_range IN ('previous_week', 'previous_2_weeks', 'month', 'previous_month')),
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
