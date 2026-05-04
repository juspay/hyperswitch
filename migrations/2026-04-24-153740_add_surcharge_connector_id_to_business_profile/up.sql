-- Add surcharge_connector_details to business_profile table
ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS surcharge_connector_details JSONB;
