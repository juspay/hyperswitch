-- This file should undo anything in `up.sql`
ALTER TABLE payment_attempt
DROP COLUMN IF EXISTS external_3ds_authentication_requested,
DROP COLUMN IF EXISTS authentication_provider,
DROP COLUMN IF EXISTS authentication_id;