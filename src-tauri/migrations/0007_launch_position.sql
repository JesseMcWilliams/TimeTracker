-- One of: top-left, top-center, top-right, middle-left, center, middle-right,
-- bottom-left, bottom-center, bottom-right, or 'default' (let the OS place the window).
ALTER TABLE user_profile ADD COLUMN launch_position TEXT NOT NULL DEFAULT 'default';
