-- Single-row user profile (this app is single-user). The CHECK pins it to id=1 so
-- there can only ever be one row; saves are an upsert against that fixed id.
CREATE TABLE user_profile (
    id            INTEGER PRIMARY KEY CHECK (id = 1),
    first_name    TEXT,
    last_name     TEXT,
    full_name     TEXT,
    email         TEXT,
    output_folder TEXT,
    output_type   TEXT NOT NULL DEFAULT 'xlsx' CHECK (output_type IN ('csv', 'xlsx'))
);

-- User-assigned business/reference IDs, independent of the internal auto-increment id.
ALTER TABLE clients ADD COLUMN external_id TEXT;
ALTER TABLE contracts ADD COLUMN external_id TEXT;

-- A client's own week boundaries, used when generating that client's contract
-- timesheets so "this week" lines up with how the client actually bills weeks.
ALTER TABLE clients ADD COLUMN week_start TEXT NOT NULL DEFAULT 'monday';
ALTER TABLE clients ADD COLUMN week_end TEXT NOT NULL DEFAULT 'sunday';

ALTER TABLE contracts ADD COLUMN start_date TEXT;
