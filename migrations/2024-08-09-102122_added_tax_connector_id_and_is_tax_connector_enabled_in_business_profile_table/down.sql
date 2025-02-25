-- This file should undo anything in `up.sql`
ALTER TABLE business_profile DROP COLUMN IF EXISTS tax_connector_id;
ALTER TABLE business_profile DROP COLUMN IF EXISTS is_tax_connector_enabled;