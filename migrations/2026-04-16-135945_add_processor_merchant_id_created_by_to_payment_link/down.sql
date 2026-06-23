-- This file should undo anything in `up.sql`
ALTER TABLE payment_link DROP COLUMN IF EXISTS processor_merchant_id;
ALTER TABLE payment_link DROP COLUMN IF EXISTS created_by;
