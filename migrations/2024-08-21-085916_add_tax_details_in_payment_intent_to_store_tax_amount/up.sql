-- Your SQL goes here
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS tax_details JSONB;
