-- Adds "this week" and "last 2 weeks including this one" to the email template date
-- ranges. The existing set only looked backwards at completed periods, so a template
-- run mid-week reported on the week before last from the sender's point of view —
-- there was no way to ask "which contracts have no time logged *this* week", which is
-- the question you actually send that email about.
--
-- date_range is constrained by a CHECK, and SQLite cannot alter one in place, so the
-- table is rebuilt. Everything else about it is unchanged, and existing rows keep their
-- current values: no template changes the period it reports on as a result of this.

CREATE TABLE email_templates_new (
    id               INTEGER PRIMARY KEY,
    name             TEXT NOT NULL,
    description      TEXT,
    notes            TEXT,
    email            TEXT,
    contract_id      INTEGER REFERENCES contracts(id),
    date_range       TEXT NOT NULL DEFAULT 'previous_week'
                     CHECK (date_range IN ('current_week', 'previous_week', 'current_2_weeks',
                                           'previous_2_weeks', 'month', 'previous_month')),
    created_at       TEXT NOT NULL DEFAULT (datetime('now')),
    subject_template TEXT,
    body_template    TEXT,
    is_default       INTEGER NOT NULL DEFAULT 0,
    date_format      TEXT NOT NULL DEFAULT 'iso'
                     CHECK (date_format IN ('iso', 'us', 'long'))
);

INSERT INTO email_templates_new
    (id, name, description, notes, email, contract_id, date_range, created_at,
     subject_template, body_template, is_default, date_format)
SELECT id, name, description, notes, email, contract_id, date_range, created_at,
       subject_template, body_template, is_default, date_format
FROM email_templates;

DROP TABLE email_templates;
ALTER TABLE email_templates_new RENAME TO email_templates;

-- Recreated with the table: at most one template may be the default.
CREATE UNIQUE INDEX idx_email_templates_single_default
    ON email_templates(is_default) WHERE is_default = 1;
