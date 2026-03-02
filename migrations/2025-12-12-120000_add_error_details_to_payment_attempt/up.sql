-- Your SQL goes here
ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS error_details JSONB;
