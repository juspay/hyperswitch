-- Your SQL goes here
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS authorization_count INTEGER;
