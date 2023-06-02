-- This file should undo anything in `up.sql`
ALTER TABLE merchant_account ALTER COLUMN locker_id DROP DEFAULT;
