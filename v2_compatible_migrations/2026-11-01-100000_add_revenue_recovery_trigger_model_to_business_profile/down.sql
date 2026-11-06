-- This file should undo anything in `up.sql`
ALTER TABLE business_profile
DROP COLUMN IF EXISTS revenue_recovery_trigger_model;
