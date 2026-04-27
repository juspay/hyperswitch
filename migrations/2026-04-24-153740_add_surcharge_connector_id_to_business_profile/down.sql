-- Remove surcharge_connector_id from business_profile table
ALTER TABLE business_profile DROP COLUMN IF EXISTS surcharge_connector_id;
