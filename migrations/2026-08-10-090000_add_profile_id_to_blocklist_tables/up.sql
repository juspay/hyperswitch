-- Your SQL goes here

ALTER TABLE blocklist ADD COLUMN IF NOT EXISTS profile_id VARCHAR(64);

ALTER TABLE batch_blocklist_jobs ADD COLUMN IF NOT EXISTS profile_id VARCHAR(64);
