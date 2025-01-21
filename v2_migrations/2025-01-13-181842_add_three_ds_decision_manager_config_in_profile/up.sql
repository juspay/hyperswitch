-- Your SQL goes here
ALTER TABLE business_profile
ADD COLUMN IF NOT EXISTS three_ds_decision_manager_config jsonb;