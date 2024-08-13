-- This file should undo anything in `up.sql`
ALTER TABLE merchant_account
ADD COLUMN return_url VARCHAR(255);

ALTER TABLE merchant_account
ADD COLUMN enable_payment_response_hash BOOLEAN DEFAULT FALSE;

ALTER TABLE merchant_account
ADD COLUMN payment_response_hash_key VARCHAR(255);

ALTER TABLE merchant_account
ADD COLUMN redirect_to_merchant_with_http_post BOOLEAN DEFAULT FALSE;

ALTER TABLE merchant_account
ADD COLUMN sub_merchants_enabled BOOLEAN DEFAULT FALSE;

ALTER TABLE merchant_account
ADD COLUMN parent_merchant_id VARCHAR(64);

-- The default value is for temporary purpose only
ALTER TABLE merchant_account
ADD COLUMN primary_business_details JSON NOT NULL DEFAULT '[{"country": "US", "business": "default"}]';

ALTER TABLE merchant_account
ALTER COLUMN primary_business_details DROP DEFAULT;

ALTER TABLE merchant_account
ADD COLUMN locker_id VARCHAR(64);

ALTER TABLE merchant_account
ADD COLUMN intent_fulfillment_time BIGINT;

ALTER TABLE merchant_account
ADD COLUMN default_profile VARCHAR(64);

ALTER TABLE merchant_account
ADD COLUMN payment_link_config JSONB NULL;

ALTER TABLE merchant_account
ADD COLUMN pm_collect_link_config JSONB NULL;

ALTER TABLE merchant_account
ADD COLUMN is_recon_enabled BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE merchant_account
ADD COLUMN webhook_details JSONB NULL;

ALTER TABLE merchant_account
ADD COLUMN routing_algorithm JSON;

ALTER TABLE merchant_account
ADD COLUMN frm_routing_algorithm JSONB;

ALTER TABLE merchant_account
ADD COLUMN payout_routing_algorithm JSONB;
