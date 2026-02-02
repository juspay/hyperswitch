-- Your SQL goes here
ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS encrypted_payment_method_data BYTEA;
