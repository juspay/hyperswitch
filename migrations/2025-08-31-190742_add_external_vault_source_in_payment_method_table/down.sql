-- This file should undo anything in `up.sql`
ALTER TABLE payment_methods DROP COLUMN IF EXISTS external_vault_source;

ALTER TABLE payment_methods DROP COLUMN IF EXISTS vault_type;

