-- Your SQL goes here
ALTER TABLE roles ADD COLUMN IF NOT EXISTS profile_id VARCHAR(64);