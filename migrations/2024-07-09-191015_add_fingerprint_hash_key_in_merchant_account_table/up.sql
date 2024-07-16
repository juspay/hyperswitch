-- Your SQL goes here
ALTER TABLE merchant_account ADD COLUMN IF NOT EXISTS fingerprint_hash_key BYTEA DEFAULT NULL;
