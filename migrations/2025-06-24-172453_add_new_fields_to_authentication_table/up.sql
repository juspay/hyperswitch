-- Your SQL goes here
ALTER TABLE authentication 
ADD COLUMN IF NOT EXISTS billing_address BYTEA,
ADD COLUMN IF NOT EXISTS shipping_address BYTEA,
ADD COLUMN IF NOT EXISTS browser_info JSONB,
ADD COLUMN IF NOT EXISTS email BYTEA;