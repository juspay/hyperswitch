-- Your SQL goes here
ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS token_payment_method_data BYTEA DEFAULT NULL;