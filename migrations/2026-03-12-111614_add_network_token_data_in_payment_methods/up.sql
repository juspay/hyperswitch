-- Your SQL goes here
ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS network_tokenization_data BYTEA;