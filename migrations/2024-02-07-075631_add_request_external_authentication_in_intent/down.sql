-- This file should undo anything in `up.sql`
ALTER TABLE payment_intent DROP COLUMN IF EXISTS request_external_three_ds_authentication;