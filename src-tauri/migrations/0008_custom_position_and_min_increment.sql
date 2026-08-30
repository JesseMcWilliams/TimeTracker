-- Exact remembered window position, used when launch_position = 'custom'.
ALTER TABLE user_profile ADD COLUMN window_x INTEGER;
ALTER TABLE user_profile ADD COLUMN window_y INTEGER;

-- Minimum billable increment in minutes for a client (e.g. 15 or 30) — entry durations
-- for that client's contracts round up to the next multiple. NULL/0 means no rounding.
ALTER TABLE clients ADD COLUMN minimum_increment_minutes INTEGER;
