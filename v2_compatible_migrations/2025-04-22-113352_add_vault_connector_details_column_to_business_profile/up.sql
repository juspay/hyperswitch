ALTER TABLE business_profile 
ADD COLUMN IF NOT EXISTS vault_connector_details JSONB;