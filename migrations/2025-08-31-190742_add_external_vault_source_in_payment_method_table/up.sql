-- Your SQL goes here
ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS external_vault_source VARCHAR(64);

ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS vault_type VARCHAR(64);