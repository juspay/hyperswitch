-- This file should undo anything in `up.sql`
ALTER TABLE merchant_account DROP COLUMN IF EXISTS network_tokenization_credentials;
ALTER TABLE business_profile DROP COLUMN IF EXISTS network_tokenization_credentials;