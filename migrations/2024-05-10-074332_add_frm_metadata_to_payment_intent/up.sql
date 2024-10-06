-- Your SQL goes here
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS frm_metadata JSONB DEFAULT NULL;