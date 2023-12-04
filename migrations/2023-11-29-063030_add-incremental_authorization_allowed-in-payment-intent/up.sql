-- Your SQL goes here
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS incremental_authorization_allowed BOOLEAN;