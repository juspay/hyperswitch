-- This file should undo anything in `up.sql`
ALTER TABLE payouts DROP COLUMN IF EXISTS processor_merchant_id;
ALTER TABLE payouts DROP COLUMN IF EXISTS created_by;

ALTER TABLE payout_attempt DROP COLUMN IF EXISTS processor_merchant_id;
ALTER TABLE payout_attempt DROP COLUMN IF EXISTS created_by;
