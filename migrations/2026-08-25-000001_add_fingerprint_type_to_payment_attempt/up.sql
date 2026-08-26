-- Your SQL goes here
ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS fingerprint_type VARCHAR(10);
