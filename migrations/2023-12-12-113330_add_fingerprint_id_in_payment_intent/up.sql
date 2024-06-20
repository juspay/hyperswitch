-- Your SQL goes here
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS fingerprint_id VARCHAR(64);
