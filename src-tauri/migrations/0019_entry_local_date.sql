-- Records which calendar day an entry belongs to, decided once when the entry is
-- written rather than re-derived from the viewer's current timezone on every query.
--
-- started_at is a UTC instant, which answers "when did this happen" but not "which day
-- does this bill to" — and every period question this app asks is the second one. Until
-- now the answer was recomputed at query time from whoever was looking, which meant an
-- entry could change which week it belonged to simply because the app was opened from a
-- different timezone, and period bounds near a DST boundary used today's offset rather
-- than the one actually in effect back then.
--
-- local_date is the civil date in the timezone the entry was recorded in, and it never
-- moves again. utc_offset_minutes records the offset east of UTC that produced it, so
-- the derivation stays auditable and the original wall-clock time is recoverable.

ALTER TABLE time_entries ADD COLUMN local_date TEXT;
ALTER TABLE time_entries ADD COLUMN utc_offset_minutes INTEGER;

-- Backfill from this machine's timezone, applying the rules in effect on each entry's
-- own date rather than one flat offset for all of history. This is a reconstruction,
-- not a record: entries logged while in a different timezone than this machine's are
-- backfilled as though they were logged here, because the offset that actually applied
-- was never stored. Nothing can recover it, and every row written from here on records
-- its own offset so this is the last time it has to be guessed.
UPDATE time_entries
SET local_date = date(started_at, 'localtime'),
    utc_offset_minutes = CAST(
        round((julianday(datetime(started_at, 'localtime')) - julianday(started_at)) * 1440) AS INTEGER
    );

-- A started_at SQLite cannot parse yields NULL above. None is expected — every writer
-- goes through chrono or JS toISOString — but a period query silently skipping rows is
-- worse than a slightly wrong date, so fall back to the UTC calendar day, which is what
-- those rows were already being filed under before this migration.
UPDATE time_entries
SET local_date = substr(started_at, 1, 10),
    utc_offset_minutes = 0
WHERE local_date IS NULL;

CREATE INDEX idx_time_entries_local_date ON time_entries(local_date);
