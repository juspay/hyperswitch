-- Your SQL goes here
ALTER TABLE payouts ADD COLUMN IF NOT EXISTS processor_merchant_id VARCHAR(64);
ALTER TABLE payouts ADD COLUMN IF NOT EXISTS created_by VARCHAR(255);

ALTER TABLE payout_attempt ADD COLUMN IF NOT EXISTS processor_merchant_id VARCHAR(64);
ALTER TABLE payout_attempt ADD COLUMN IF NOT EXISTS created_by VARCHAR(255);
