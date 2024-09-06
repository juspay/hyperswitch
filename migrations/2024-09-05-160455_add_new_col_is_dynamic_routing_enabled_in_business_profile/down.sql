-- This file should undo anything in `up.sql`
ALTER TABLE business_profile
DROP COLUMN is_dynamic_routing_enabled;
