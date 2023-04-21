-- This file should undo anything in `up.sql`
ALTER TABLE merchant_account
DROP COLUMN IF EXISTS created_at,
DROP COLUMN IF EXISTS modified_at;


ALTER TABLE merchant_connector_account
DROP COLUMN IF EXISTS created_at,
DROP COLUMN IF EXISTS modified_at;

ALTER TABLE customers
DROP COLUMN IF EXISTS modified_at;
