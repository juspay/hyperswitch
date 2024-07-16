-- Your SQL goes here

ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS locker_id VARCHAR(64) DEFAULT NULL;