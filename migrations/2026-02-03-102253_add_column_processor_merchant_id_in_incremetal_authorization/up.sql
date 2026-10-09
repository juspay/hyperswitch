-- Your SQL goes here
ALTER TABLE incremental_authorization ADD COLUMN IF NOT EXISTS processor_merchant_id VARCHAR(64);
-- This backfill should be executed again after deployment
UPDATE incremental_authorization SET processor_merchant_id = merchant_id where processor_merchant_id IS NULL;