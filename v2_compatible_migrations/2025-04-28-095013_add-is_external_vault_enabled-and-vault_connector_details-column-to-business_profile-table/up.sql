-- Your SQL goes here
ALTER TABLE business_profile
ADD COLUMN IF NOT EXISTS is_external_vault_enabled BOOLEAN DEFAULT FALSE;

ALTER TABLE business_profile 
ADD COLUMN IF NOT EXISTS vault_connector_details JSONB;