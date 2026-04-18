-- Your SQL goes here
ALTER TABLE payment_link ADD COLUMN IF NOT EXISTS processor_merchant_id VARCHAR(64);
ALTER TABLE payment_link ADD COLUMN IF NOT EXISTS created_by VARCHAR(255);

-- This backfill should be executed after deployment is complete.
UPDATE payment_link SET processor_merchant_id = merchant_id WHERE processor_merchant_id IS NULL;
