-- Your SQL goes here
ALTER TABLE business_profile
ADD COLUMN IF NOT EXISTS payment_link_config JSONB DEFAULT NULL;
