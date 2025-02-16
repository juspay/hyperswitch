-- Your SQL goes here
ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS is_clear_pan_retries_enabled BOOLEAN NOT NULL DEFAULT FALSE;