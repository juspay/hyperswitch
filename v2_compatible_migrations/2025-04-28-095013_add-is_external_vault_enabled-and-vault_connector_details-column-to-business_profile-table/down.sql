-- This file should undo anything in `up.sql`
-- This file should undo anything in `up.sql`
ALTER TABLE business_profile DROP COLUMN IF EXISTS is_external_vault_enabled;

ALTER TABLE business_profile DROP COLUMN IF EXISTS external_vault_connector_details;