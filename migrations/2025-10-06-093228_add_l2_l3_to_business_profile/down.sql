-- This file should undo anything in `up.sql`
ALTER TABLE business_profile DROP COLUMN IF EXISTS is_l2_l3_enabled;
