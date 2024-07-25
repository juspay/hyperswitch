-- Your SQL goes here
ALTER TABLE customers ADD COLUMN IF NOT EXISTS default_billing_address BYTEA DEFAULT NULL;
