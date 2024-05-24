-- Your SQL goes here
ALTER TABLE authentication ADD COLUMN IF NOT EXISTS directory_server_id VARCHAR(128);