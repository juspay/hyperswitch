-- Your SQL goes here
ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS is_network_tokenization_enabled BOOLEAN NOT NULL DEFAULT FALSE;