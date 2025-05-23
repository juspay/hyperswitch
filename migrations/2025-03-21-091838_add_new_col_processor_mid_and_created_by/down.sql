-- This file should undo anything in `up.sql`
ALTER TABLE payment_intent DROP COLUMN IF EXISTS processor_merchant_id;
ALTER TABLE payment_intent DROP COLUMN IF EXISTS created_by;
ALTER TABLE payment_attempt DROP COLUMN IF EXISTS processor_merchant_id;
ALTER TABLE payment_attempt DROP COLUMN IF EXISTS created_by;
