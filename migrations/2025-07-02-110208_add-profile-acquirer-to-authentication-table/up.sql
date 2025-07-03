-- Your SQL goes here
ALTER TABLE authentication
    ADD COLUMN IF NOT EXISTS profile_acquirer_id VARCHAR(128) NULL;