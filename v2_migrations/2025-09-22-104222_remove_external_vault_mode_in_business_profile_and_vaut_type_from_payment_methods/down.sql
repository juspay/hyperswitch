-- This file should undo anything in `up.sql`
ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS external_vault_mode VARCHAR(16);

-- This file should undo anything in `up.sql`
ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS vault_type VARCHAR(64);