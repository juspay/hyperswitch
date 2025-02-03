-- Your SQL goes here
ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS platform_merchant_id VARCHAR(64) NULL;
