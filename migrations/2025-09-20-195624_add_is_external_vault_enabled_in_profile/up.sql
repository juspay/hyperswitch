-- Your SQL goes here
ALTER TABLE business_profile
ADD COLUMN IF NOT EXISTS external_vault_mode VARCHAR(16);

ALTER TABLE business_profile 
ADD COLUMN IF NOT EXISTS external_vault_connector_details JSONB;