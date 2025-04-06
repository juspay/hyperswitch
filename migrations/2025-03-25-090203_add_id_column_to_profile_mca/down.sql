-- This file should undo anything in `up.sql`
ALTER TABLE business_profile
DROP COLUMN id;

ALTER TABLE merchant_connector_account
DROP COLUMN id;