-- Your SQL goes here
ALTER TABLE events ADD COLUMN IF NOT EXISTS recipient VARCHAR(32);