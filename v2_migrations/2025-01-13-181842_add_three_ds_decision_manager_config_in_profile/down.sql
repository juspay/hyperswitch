-- This file should undo anything in `up.sql`
ALTER TABLE business_profile
DROP COLUMN IF EXISTS three_ds_decision_manager_config;