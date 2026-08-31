-- Named, reusable column-name mappings for bulk import, so a spreadsheet using
-- different header names (e.g. "Work Date"/"Clock In"/"Clock Out") doesn't need its
-- headers renamed before importing. category_column/notes_column are nullable —
-- leaving one blank means "don't look for that column at all" for this template.
CREATE TABLE import_templates (
    id              INTEGER PRIMARY KEY,
    name            TEXT NOT NULL,
    notes           TEXT,
    is_default      INTEGER NOT NULL DEFAULT 0,
    date_column     TEXT NOT NULL DEFAULT 'Date',
    start_column    TEXT NOT NULL DEFAULT 'Start Time',
    end_column      TEXT NOT NULL DEFAULT 'End Time',
    category_column TEXT,
    notes_column    TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Two starter templates so importing works immediately without configuring anything
-- first: the app's own historical column names, and the common "Activity" variant
-- seen in real-world timesheets (Date/Start/End/Category/Activity).
INSERT INTO import_templates
    (name, notes, is_default, date_column, start_column, end_column, category_column, notes_column)
VALUES
    ('Default', 'Date, Start Time, End Time, Category, Notes.', 1,
     'Date', 'Start Time', 'End Time', 'Category', 'Notes'),
    ('Activity Sheet', 'Common variant using Start/End/Activity column names.', 0,
     'Date', 'Start', 'End', 'Category', 'Activity');
