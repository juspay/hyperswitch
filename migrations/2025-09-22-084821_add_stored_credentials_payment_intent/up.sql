-- Your SQL goes here
ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS is_stored_credential BOOLEAN;
