-- Your SQL goes here
ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS client_source VARCHAR(64) DEFAULT NULL;
ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS client_version VARCHAR(64) DEFAULT NULL;