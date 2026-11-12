-- Your SQL goes here
ALTER TABLE business_profile
ADD COLUMN IF NOT EXISTS revenue_recovery_config JSONB;
