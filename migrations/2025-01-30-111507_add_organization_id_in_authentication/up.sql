-- Your SQL goes here
ALTER TABLE authentication
    ADD COLUMN IF NOT EXISTS organization_id VARCHAR(32) NOT NULL DEFAULT 'default_org';