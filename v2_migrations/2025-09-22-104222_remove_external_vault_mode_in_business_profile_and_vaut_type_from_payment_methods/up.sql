-- Remove the external_vault_mode column from business_profile table
-- Your SQL goes here
ALTER TABLE business_profile DROP COLUMN IF EXISTS external_vault_mode;

-- Remove the vault_type column from payment_methods table
-- Your SQL goes here
ALTER TABLE payment_methods DROP COLUMN IF EXISTS vault_type;