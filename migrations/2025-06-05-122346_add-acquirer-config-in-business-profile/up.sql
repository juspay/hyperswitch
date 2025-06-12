-- Your SQL goes here
ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS acquirer_config_map JSONB;