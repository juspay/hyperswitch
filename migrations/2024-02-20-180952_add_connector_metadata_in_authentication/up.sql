-- Your SQL goes here
ALTER TABLE authentication ADD COLUMN IF NOT EXISTS connector_metadata JSONB DEFAULT NULL;
