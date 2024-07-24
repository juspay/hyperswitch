-- Your SQL goes here
ALTER TABLE customers ADD COLUMN IF NOT EXISTS default_billing_address VARCHAR(255);