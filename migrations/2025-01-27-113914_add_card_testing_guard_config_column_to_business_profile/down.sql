-- This file should undo anything in `up.sql`

ALTER TABLE business_profile 
DROP COLUMN IF EXISTS card_testing_guard_config;

ALTER TABLE business_profile 
DROP COLUMN IF EXISTS card_testing_secret_key;
