-- Your SQL goes here
ALTER TABLE authentication 
ADD COLUMN IF NOT EXISTS billing_address BYTEA,
ADD COLUMN IF NOT EXISTS shipping_address BYTEA,
ADD COLUMN IF NOT EXISTS browser_info JSONB,
ADD COLUMN IF NOT EXISTS email BYTEA,
ADD COLUMN IF NOT EXISTS amount bigint,
ADD COLUMN IF NOT EXISTS currency "Currency",
ADD COLUMN IF NOT EXISTS profile_acquirer_id VARCHAR(128) NULL;