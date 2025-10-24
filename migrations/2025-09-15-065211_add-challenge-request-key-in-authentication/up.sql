-- Your SQL goes here
ALTER TABLE authentication ADD COLUMN IF NOT EXISTS challenge_request_key VARCHAR(255);
