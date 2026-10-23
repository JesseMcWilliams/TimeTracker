-- Email templates gain author-controlled subject and body text, a default-template
-- flag, and a date format. Both *_template columns are NULL for templates created
-- before this migration, which means "use the built-in wording" — so existing
-- templates keep generating exactly the email they did before.
ALTER TABLE email_templates ADD COLUMN subject_template TEXT;
ALTER TABLE email_templates ADD COLUMN body_template TEXT;
ALTER TABLE email_templates ADD COLUMN is_default INTEGER NOT NULL DEFAULT 0;
ALTER TABLE email_templates ADD COLUMN date_format TEXT NOT NULL DEFAULT 'iso'
    CHECK (date_format IN ('iso', 'us', 'long'));

-- At most one template may be the default. A partial unique index lets any number of
-- rows hold 0 while allowing only a single 1, so the constraint is enforced by the
-- database rather than by remembering to clear the old default in application code.
CREATE UNIQUE INDEX idx_email_templates_single_default
    ON email_templates(is_default) WHERE is_default = 1;
