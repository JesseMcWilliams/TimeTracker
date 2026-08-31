-- Import Templates now support multiple alias column names per field (e.g. a Start
-- field matching either "Start" or "Start Time"), tried in order — the first alias
-- found in the file's header row wins. Values are comma-separated; every existing
-- single-name value already works unchanged as a one-item list. Renaming rather than
-- adding new columns keeps exactly one column per field instead of two parallel sets.
ALTER TABLE import_templates RENAME COLUMN date_column TO date_columns;
ALTER TABLE import_templates RENAME COLUMN start_column TO start_columns;
ALTER TABLE import_templates RENAME COLUMN end_column TO end_columns;
ALTER TABLE import_templates RENAME COLUMN category_column TO category_columns;
ALTER TABLE import_templates RENAME COLUMN notes_column TO notes_columns;

-- Give the seeded "Activity Sheet" template a couple of extra aliases as a working
-- example of the feature, since its Start/End columns are the ones most likely to
-- vary by source spreadsheet.
UPDATE import_templates
SET start_columns = 'Start, Start Time',
    end_columns = 'End, End Time'
WHERE name = 'Activity Sheet';
