-- Your SQL goes here
ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS network_token_locker_id VARCHAR(64) DEFAULT NULL;