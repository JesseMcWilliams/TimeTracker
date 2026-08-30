ALTER TABLE user_profile ADD COLUMN window_width INTEGER;
ALTER TABLE user_profile ADD COLUMN window_height INTEGER;
ALTER TABLE user_profile ADD COLUMN default_start_page TEXT NOT NULL DEFAULT 'timer';

-- A client's default category, used by the Quick Timer page so starting a timer there
-- doesn't require picking a category every time for clients that have one obvious default.
ALTER TABLE clients ADD COLUMN default_tracking_code_id INTEGER REFERENCES tracking_codes(id);
