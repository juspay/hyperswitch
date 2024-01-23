-- Your SQL goes here
ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS external_3ds_authentication_requested BOOLEAN default false,
ADD COLUMN IF NOT EXISTS authentication_provider VARCHAR(64),
ADD COLUMN IF NOT EXISTS authentication_id VARCHAR(64);