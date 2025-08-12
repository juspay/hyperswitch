-- Your SQL goes here
ALTER TABLE business_profile
ADD COLUMN IF NOT EXISTS billing_connector VARCHAR(255);