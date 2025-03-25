-- This file drops all new columns being added as part of v2 refactoring.
-- These migrations can be run as long as there's no v2 application running.
ALTER TABLE customers DROP COLUMN IF EXISTS merchant_reference_id,
    DROP COLUMN IF EXISTS default_billing_address,
    DROP COLUMN IF EXISTS default_shipping_address,
    DROP COLUMN IF EXISTS status;

ALTER TABLE business_profile DROP COLUMN routing_algorithm_id,
    DROP COLUMN order_fulfillment_time,
    DROP COLUMN order_fulfillment_time_origin,
    DROP COLUMN frm_routing_algorithm_id,
    DROP COLUMN payout_routing_algorithm_id,
    DROP COLUMN default_fallback_routing,
    DROP COLUMN should_collect_cvv_during_payment,
    DROP COLUMN three_ds_decision_manager_config;

DROP TYPE "OrderFulfillmentTimeOrigin";

-- Revert renaming of field,
ALTER TABLE payment_intent DROP COLUMN merchant_reference_id,
    DROP COLUMN billing_address,
    DROP COLUMN shipping_address,
    DROP COLUMN capture_method,
    DROP COLUMN authentication_type,
    DROP COLUMN amount_to_capture,
    DROP COLUMN prerouting_algorithm,
    DROP COLUMN surcharge_amount,
    DROP COLUMN tax_on_surcharge,
    DROP COLUMN frm_merchant_decision,
    DROP COLUMN statement_descriptor,
    DROP COLUMN enable_payment_link,
    DROP COLUMN apply_mit_exemption,
    DROP COLUMN customer_present,
    DROP COLUMN routing_algorithm_id,
    DROP COLUMN payment_link_config;

ALTER TABLE payment_attempt DROP COLUMN payment_method_type_v2,
    DROP COLUMN connector_payment_id,
    DROP COLUMN payment_method_subtype,
    DROP COLUMN routing_result,
    DROP COLUMN authentication_applied,
    DROP COLUMN external_reference_id,
    DROP COLUMN tax_on_surcharge,
    DROP COLUMN payment_method_billing_address,
    DROP COLUMN redirection_data,
    DROP COLUMN connector_payment_data,
    DROP COLUMN connector_token_details;

ALTER TABLE merchant_connector_account
    DROP COLUMN IF EXISTS feature_metadata;

ALTER TABLE payment_methods
    DROP COLUMN IF EXISTS locker_fingerprint_id,
    DROP COLUMN IF EXISTS payment_method_type_v2,
    DROP COLUMN IF EXISTS payment_method_subtype;

ALTER TABLE refund
    DROP COLUMN IF EXISTS id,
    DROP COLUMN IF EXISTS merchant_reference_id,
    DROP COLUMN IF EXISTS connector_id;