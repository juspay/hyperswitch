-- This file contains all new columns being added as part of v2 refactoring.
-- The new columns added should work with both v1 and v2 applications.
ALTER TABLE customers
ADD COLUMN IF NOT EXISTS merchant_reference_id VARCHAR(64),
    ADD COLUMN IF NOT EXISTS default_billing_address BYTEA DEFAULT NULL,
    ADD COLUMN IF NOT EXISTS default_shipping_address BYTEA DEFAULT NULL,
    ADD COLUMN IF NOT EXISTS status "DeleteStatus" NOT NULL DEFAULT 'active';

CREATE TYPE "OrderFulfillmentTimeOrigin" AS ENUM ('create', 'confirm');

ALTER TABLE business_profile
ADD COLUMN routing_algorithm_id VARCHAR(64) DEFAULT NULL,
    ADD COLUMN order_fulfillment_time BIGINT DEFAULT NULL,
    ADD COLUMN order_fulfillment_time_origin "OrderFulfillmentTimeOrigin" DEFAULT NULL,
    ADD COLUMN frm_routing_algorithm_id VARCHAR(64) DEFAULT NULL,
    ADD COLUMN payout_routing_algorithm_id VARCHAR(64) DEFAULT NULL,
    ADD COLUMN default_fallback_routing JSONB DEFAULT NULL;

ALTER TABLE payment_intent
ADD COLUMN merchant_reference_id VARCHAR(64),
    ADD COLUMN billing_address BYTEA DEFAULT NULL,
    ADD COLUMN shipping_address BYTEA DEFAULT NULL,
    ADD COLUMN capture_method "CaptureMethod",
    ADD COLUMN authentication_type "AuthenticationType",
    ADD COLUMN amount_to_capture bigint,
    ADD COLUMN prerouting_algorithm JSONB,
    ADD COLUMN surcharge_amount bigint,
    ADD COLUMN tax_on_surcharge bigint,
    ADD COLUMN frm_merchant_decision VARCHAR(64),
    ADD COLUMN statement_descriptor VARCHAR(255),
    ADD COLUMN enable_payment_link BOOLEAN,
    ADD COLUMN apply_mit_exemption BOOLEAN,
    ADD COLUMN customer_present BOOLEAN,
    ADD COLUMN routing_algorithm_id VARCHAR(64),
    ADD COLUMN payment_link_config JSONB;
