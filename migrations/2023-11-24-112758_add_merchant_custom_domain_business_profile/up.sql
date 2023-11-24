-- Your SQL goes here
ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS merchant_custom_domain VARCHAR(256) NULL;