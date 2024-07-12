-- This file should undo anything in `up.sql`
ALTER TABLE merchant_account
ALTER COLUMN id
SET NOT NULL;
