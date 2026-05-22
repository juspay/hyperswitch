-- Your SQL goes here
ALTER TABLE merchant_account ADD COLUMN IF NOT EXISTS network_tokenization_credentials BYTEA;
ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS network_tokenization_credentials BYTEA;