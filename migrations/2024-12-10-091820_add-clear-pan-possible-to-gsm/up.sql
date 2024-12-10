-- Your SQL goes here
ALTER TABLE gateway_status_map ADD COLUMN IF NOT EXISTS clear_pan_possible BOOLEAN NOT NULL DEFAULT FALSE;