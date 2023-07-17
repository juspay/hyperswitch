-- Your SQL goes here
ALTER TABLE merchant_account
ADD COLUMN IF NOT EXISTS organization_id VARCHAR(32);
