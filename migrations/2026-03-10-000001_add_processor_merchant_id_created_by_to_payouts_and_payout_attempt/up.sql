-- Your SQL goes here
ALTER TABLE payouts ADD COLUMN IF NOT EXISTS processor_merchant_id VARCHAR(64);
ALTER TABLE payouts ADD COLUMN IF NOT EXISTS created_by VARCHAR(255);

ALTER TABLE payout_attempt ADD COLUMN IF NOT EXISTS processor_merchant_id VARCHAR(64);
ALTER TABLE payout_attempt ADD COLUMN IF NOT EXISTS created_by VARCHAR(255);

-- This backfill should be executed after deployment is complete.  
UPDATE payouts SET processor_merchant_id = merchant_id WHERE processor_merchant_id IS NULL; 
UPDATE payout_attempt SET processor_merchant_id = merchant_id WHERE processor_merchant_id IS NULL; 
