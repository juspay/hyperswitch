ALTER TABLE ORGANIZATION
ADD COLUMN org_id VARCHAR(32),
    ADD COLUMN org_name TEXT;

ALTER TABLE merchant_account
ADD COLUMN merchant_id VARCHAR(64),
    ADD COLUMN return_url VARCHAR(255),
    ADD COLUMN enable_payment_response_hash BOOLEAN DEFAULT FALSE,
    ADD COLUMN payment_response_hash_key VARCHAR(255),
    ADD COLUMN redirect_to_merchant_with_http_post BOOLEAN DEFAULT FALSE,
    ADD COLUMN sub_merchants_enabled BOOLEAN DEFAULT FALSE,
    ADD COLUMN parent_merchant_id VARCHAR(64),
    ADD COLUMN locker_id VARCHAR(64),
    ADD COLUMN intent_fulfillment_time BIGINT,
    ADD COLUMN default_profile VARCHAR(64),
    ADD COLUMN payment_link_config JSONB NULL,
    ADD COLUMN pm_collect_link_config JSONB NULL,
    ADD COLUMN is_recon_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN webhook_details JSONB NULL,
    ADD COLUMN routing_algorithm JSON,
    ADD COLUMN frm_routing_algorithm JSONB,
    ADD COLUMN payout_routing_algorithm JSONB;

-- The default value is for temporary purpose only
ALTER TABLE merchant_account
ADD COLUMN primary_business_details JSON NOT NULL DEFAULT '[{"country": "US", "business": "default"}]';

ALTER TABLE merchant_account
ALTER COLUMN primary_business_details DROP DEFAULT;

ALTER TABLE business_profile
ADD COLUMN profile_id VARCHAR(64),
    ADD COLUMN routing_algorithm JSON DEFAULT NULL,
    ADD COLUMN intent_fulfillment_time BIGINT DEFAULT NULL,
    ADD COLUMN frm_routing_algorithm JSONB DEFAULT NULL,
    ADD COLUMN payout_routing_algorithm JSONB DEFAULT NULL;

ALTER TABLE merchant_connector_account
ADD COLUMN IF NOT EXISTS business_country "CountryAlpha2",
    ADD COLUMN IF NOT EXISTS business_label VARCHAR(255),
    ADD COLUMN IF NOT EXISTS business_sub_label VARCHAR(64),
    ADD COLUMN IF NOT EXISTS test_mode BOOLEAN,
    ADD COLUMN IF NOT EXISTS frm_configs jsonb,
    ADD COLUMN IF NOT EXISTS merchant_connector_id VARCHAR(32);

ALTER TABLE customers
ADD COLUMN customer_id VARCHAR(64),
    ADD COLUMN address_id VARCHAR(64);

ALTER TABLE payment_intent
ADD COLUMN IF NOT EXISTS payment_id VARCHAR(64) NOT NULL,
    ADD COLUMN connector_id VARCHAR(64),
    ADD COLUMN shipping_address_id VARCHAR(64),
    ADD COLUMN billing_address_id VARCHAR(64),
    ADD COLUMN shipping_details VARCHAR(64),
    ADD COLUMN billing_details VARCHAR(64),
    ADD COLUMN statement_descriptor_suffix VARCHAR(255),
    ADD COLUMN business_country "CountryAlpha2",
    ADD COLUMN business_label VARCHAR(64),
    ADD COLUMN incremental_authorization_allowed BOOLEAN,
    ADD COLUMN merchant_decision VARCHAR(64),
    ADD COLUMN fingerprint_id VARCHAR(64),
    ADD COLUMN statement_descriptor_name VARCHAR(255),
    ADD COLUMN amount_to_capture BIGINT,
    ADD COLUMN off_session BOOLEAN,
    ADD COLUMN payment_confirm_source "PaymentSource",
    ADD COLUMN merchant_order_reference_id VARCHAR(255),
    ADD COLUMN is_payment_processor_token_flow BOOLEAN,
    ADD COLUMN charges jsonb;

ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS attempt_id VARCHAR(64) NOT NULL,
    ADD COLUMN amount bigint NOT NULL,
    ADD COLUMN currency "Currency",
    ADD COLUMN save_to_locker BOOLEAN,
    ADD COLUMN offer_amount bigint,
    ADD COLUMN payment_method VARCHAR,
    ADD COLUMN connector_transaction_id VARCHAR(64),
    ADD COLUMN connector_transaction_data VARCHAR(512),
    ADD COLUMN capture_method "CaptureMethod",
    ADD COLUMN capture_on TIMESTAMP,
    ADD COLUMN mandate_id VARCHAR(64),
    ADD COLUMN payment_method_type VARCHAR(64),
    ADD COLUMN business_sub_label VARCHAR(64),
    ADD COLUMN mandate_details JSONB,
    ADD COLUMN mandate_data JSONB,
    ADD COLUMN tax_amount bigint,
    ADD COLUMN straight_through_algorithm JSONB,
    ADD COLUMN confirm BOOLEAN,
    ADD COLUMN authentication_data JSONB,
    ADD COLUMN payment_method_billing_address_id VARCHAR(64),
    ADD COLUMN connector_mandate_detail JSONB,
    ADD COLUMN charge_id VARCHAR(64);

-- Create the index which was dropped because of dropping the column
CREATE INDEX payment_attempt_connector_transaction_id_merchant_id_index ON payment_attempt (connector_transaction_id, merchant_id);

CREATE UNIQUE INDEX payment_attempt_payment_id_merchant_id_attempt_id_index ON payment_attempt (payment_id, merchant_id, attempt_id);
