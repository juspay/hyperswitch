-- Your SQL goes here
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS session_expiry TIMESTAMP DEFAULT NULL;
