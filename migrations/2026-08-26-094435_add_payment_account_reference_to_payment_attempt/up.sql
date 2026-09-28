-- Your SQL goes here
ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS payment_account_reference VARCHAR(255);
