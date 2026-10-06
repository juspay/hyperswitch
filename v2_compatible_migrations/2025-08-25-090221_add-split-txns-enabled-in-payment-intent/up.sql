-- Your SQL goes here
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS split_txns_enabled VARCHAR(16);