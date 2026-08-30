-- Tracks the prime contractor / subcontractor names associated with a client
-- engagement (e.g. government contracting chains), independent free-text fields.
ALTER TABLE clients ADD COLUMN prime TEXT;
ALTER TABLE clients ADD COLUMN sub TEXT;
