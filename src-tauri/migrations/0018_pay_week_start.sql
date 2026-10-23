-- The Entries page's This week / Last week / Last 2 weeks filters resolve against a pay
-- week rather than a rolling 7-day window. That list spans every client, and clients can
-- each start their week on a different day, so a single fallback is needed for the
-- unfiltered case; a contract filter narrows it to that client's own week_start.
ALTER TABLE user_profile ADD COLUMN pay_week_start TEXT NOT NULL DEFAULT 'monday';
