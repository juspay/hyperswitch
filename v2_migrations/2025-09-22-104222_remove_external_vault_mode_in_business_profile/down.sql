-- This file should undo anything in `up.sql`
ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS external_vault_mode VARCHAR(16);