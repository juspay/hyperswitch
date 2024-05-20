-- Your SQL goes here
ALTER TABLE business_profile
ADD COLUMN IF NOT EXISTS authentication_connector_details JSONB NULL;
