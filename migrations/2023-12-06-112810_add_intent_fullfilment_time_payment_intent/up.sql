-- Your SQL goes here
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS expiry TIMESTAMP NOT NULL;