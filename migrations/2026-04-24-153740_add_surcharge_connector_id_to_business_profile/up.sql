-- Add surcharge_connector_id to business_profile table
ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS surcharge_connector_id VARCHAR(64);
