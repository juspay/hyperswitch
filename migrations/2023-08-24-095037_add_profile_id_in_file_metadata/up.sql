-- Your SQL goes here
ALTER TABLE file_metadata
ADD COLUMN IF NOT EXISTS profile_id VARCHAR(64);
