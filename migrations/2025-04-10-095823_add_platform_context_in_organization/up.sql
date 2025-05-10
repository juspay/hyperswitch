-- Your SQL goes here
ALTER TABLE organization ADD COLUMN IF NOT EXISTS organization_type VARCHAR(64);
ALTER TABLE organization ADD COLUMN IF NOT EXISTS platform_merchant_id VARCHAR(64);

ALTER TABLE merchant_account ADD COLUMN IF NOT EXISTS merchant_account_type VARCHAR(64);
