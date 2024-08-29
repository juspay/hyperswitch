-- Your SQL goes here

ALTER TABLE merchant_account ADD COLUMN IF NOT EXISTS is_network_tokenization_enabled BOOLEAN NOT NULL DEFAULT FALSE;