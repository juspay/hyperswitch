-- Your SQL goes here
ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS customer_acceptance JSONB DEFAULT NULL;
