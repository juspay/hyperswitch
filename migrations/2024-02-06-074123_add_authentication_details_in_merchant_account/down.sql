-- This file should undo anything in `up.sql`
ALTER TABLE merchant_account
DROP COLUMN IF EXISTS authentication_details;