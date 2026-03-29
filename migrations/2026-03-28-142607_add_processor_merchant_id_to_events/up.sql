ALTER TABLE events ADD COLUMN IF NOT EXISTS processor_merchant_id VARCHAR(64);

-- This backfill should be executed after deployment is complete.
UPDATE events SET processor_merchant_id = merchant_id WHERE processor_merchant_id IS NULL;