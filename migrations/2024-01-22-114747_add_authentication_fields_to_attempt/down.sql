-- This file should undo anything in `up.sql`
ALTER TABLE payment_attempt
DROP COLUMN IF EXISTS external_three_ds_authentication_attempted,
DROP COLUMN IF EXISTS authentication_connector,
DROP COLUMN IF EXISTS authentication_id;