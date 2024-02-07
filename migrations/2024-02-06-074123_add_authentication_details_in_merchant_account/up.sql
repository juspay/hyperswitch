-- Your SQL goes here
ALTER TABLE merchant_account
ADD COLUMN IF NOT EXISTS authentication_details JSONB NULL;
