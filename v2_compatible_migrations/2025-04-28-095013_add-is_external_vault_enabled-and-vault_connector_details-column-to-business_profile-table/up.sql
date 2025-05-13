-- Your SQL goes here
ALTER TABLE business_profile
ADD COLUMN IF NOT EXISTS is_external_vault_enabled BOOLEAN;

ALTER TABLE business_profile 
ADD COLUMN IF NOT EXISTS external_vault_connector_details JSONB;