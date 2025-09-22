-- This file should undo anything in `up.sql`
ALTER TABLE payment_methods DROP COLUMN IF EXISTS external_vault_source;

ALTER TABLE payment_methods DROP COLUMN IF EXISTS vault_type;

-- This file should undo anything in `up.sql`
ALTER TABLE business_profile DROP COLUMN IF EXISTS external_vault_mode;

ALTER TABLE business_profile DROP COLUMN IF EXISTS external_vault_connector_details;