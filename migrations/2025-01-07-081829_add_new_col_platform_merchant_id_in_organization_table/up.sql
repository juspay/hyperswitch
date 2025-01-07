-- Your SQL goes here
ALTER TABLE organization
ADD COLUMN IF NOT EXISTS platform_merchant_id VARCHAR(64);