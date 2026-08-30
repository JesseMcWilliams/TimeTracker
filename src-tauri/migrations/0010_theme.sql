-- Color theme: 'system' (follow OS light/dark), 'light', 'dark', or 'custom'.
ALTER TABLE user_profile ADD COLUMN theme TEXT NOT NULL DEFAULT 'system';
ALTER TABLE user_profile ADD COLUMN custom_bg TEXT;
ALTER TABLE user_profile ADD COLUMN custom_text TEXT;
ALTER TABLE user_profile ADD COLUMN custom_button_bg TEXT;
ALTER TABLE user_profile ADD COLUMN custom_button_text TEXT;
