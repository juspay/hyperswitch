-- Your SQL goes here

ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS is_connector_agnostic_mit_enabled BOOLEAN DEFAULT FALSE;