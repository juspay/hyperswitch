-- Your SQL goes here
ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS unified_code VARCHAR(255);
ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS unified_message VARCHAR(1024);