-- Your SQL goes here
ALTER TABLE merchant_account ADD COLUMN IF NOT EXISTS is_platform_account BOOL NOT NULL DEFAULT FALSE;

ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS platform_merchant_id VARCHAR(64);
