-- Time entries are soft-deleted so they can be recovered from a "trash" view
-- instead of being destroyed immediately.
ALTER TABLE time_entries ADD COLUMN deleted_at TEXT;
