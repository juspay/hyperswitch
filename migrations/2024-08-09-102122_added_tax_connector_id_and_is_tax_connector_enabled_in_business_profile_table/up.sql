-- Your SQL goes here
ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS tax_connector_id VARCHAR(64);
ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS is_tax_connector_enabled BOOLEAN;