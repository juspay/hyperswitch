-- Your SQL goes here
ALTER TABLE business_profile
ADD COLUMN IF NOT EXISTS product_authentication_ids JSONB NULL;
