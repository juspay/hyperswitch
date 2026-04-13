-- This file should undo anything in `up.sql`
ALTER TABLE fraud_check DROP COLUMN IF EXISTS processor_merchant_id;
ALTER TABLE fraud_check DROP COLUMN IF EXISTS created_by;
