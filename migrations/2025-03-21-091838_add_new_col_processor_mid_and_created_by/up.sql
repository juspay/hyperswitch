-- Your SQL goes here
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS processor_merchant_id VARCHAR(64);
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS created_by VARCHAR(255);
ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS processor_merchant_id VARCHAR(64);
ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS created_by VARCHAR(255);
-- This backfill should be executed again after deployment.
UPDATE payment_intent SET processor_merchant_id = merchant_id where processor_merchant_id IS NULL;
UPDATE payment_attempt SET processor_merchant_id = merchant_id where processor_merchant_id IS NULL;
