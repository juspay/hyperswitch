-- Your SQL goes here
ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS is_tokenize_before_payment_enabled BOOLEAN NOT NULL DEFAULT FALSE;