-- Your SQL goes here
ALTER TABLE payment_link ADD COLUMN IF NOT EXISTS  profile_id VARCHAR(64) DEFAULT NULL;
