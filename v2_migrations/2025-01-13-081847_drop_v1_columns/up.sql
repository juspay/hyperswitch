-- This file contains queries to drop columns no longer used by the v2 application.
-- It is safer to take a backup of the database before running these queries as they're destructive in nature.
-- These queries should only be run when we're sure that data inserted by the v1 application is no longer required.
ALTER TABLE ORGANIZATION DROP COLUMN org_id,
    DROP COLUMN org_name;

-- Note: This query should not be run on higher environments as this leads to data loss
-- The application will work fine even without these queries not being run
ALTER TABLE merchant_account DROP COLUMN merchant_id,
    DROP COLUMN return_url,
    DROP COLUMN enable_payment_response_hash,
    DROP COLUMN payment_response_hash_key,
    DROP COLUMN redirect_to_merchant_with_http_post,
    DROP COLUMN sub_merchants_enabled,
    DROP COLUMN parent_merchant_id,
    DROP COLUMN primary_business_details,
    DROP COLUMN locker_id,
    DROP COLUMN intent_fulfillment_time,
    DROP COLUMN default_profile,
    DROP COLUMN payment_link_config,
    DROP COLUMN pm_collect_link_config,
    DROP COLUMN is_recon_enabled,
    DROP COLUMN webhook_details,
    DROP COLUMN routing_algorithm,
    DROP COLUMN frm_routing_algorithm,
    DROP COLUMN payout_routing_algorithm;

-- Note: This query should not be run on higher environments as this leads to data loss.
-- The application will work fine even without these queries being run.
ALTER TABLE business_profile DROP COLUMN profile_id,
    DROP COLUMN routing_algorithm,
    DROP COLUMN intent_fulfillment_time,
    DROP COLUMN frm_routing_algorithm,
    DROP COLUMN payout_routing_algorithm;

-- This migration is to remove the fields that are no longer used by the v1  application, or some type changes.
ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS business_country,
    DROP COLUMN IF EXISTS business_label,
    DROP COLUMN IF EXISTS business_sub_label,
    DROP COLUMN IF EXISTS test_mode,
    DROP COLUMN IF EXISTS merchant_connector_id,
    DROP COLUMN IF EXISTS frm_configs;

-- Run this query only when V1 is deprecated
ALTER TABLE customers DROP COLUMN customer_id,
    DROP COLUMN address_id;

-- Run below queries only when V1 is deprecated
ALTER TABLE payment_intent DROP COLUMN payment_id,
    DROP COLUMN connector_id,
    DROP COLUMN shipping_address_id,
    DROP COLUMN billing_address_id,
    DROP COLUMN shipping_details,
    DROP COLUMN billing_details,
    DROP COLUMN statement_descriptor_suffix,
    DROP COLUMN business_country,
    DROP COLUMN business_label,
    DROP COLUMN incremental_authorization_allowed,
    DROP COLUMN fingerprint_id,
    DROP COLUMN merchant_decision,
    DROP COLUMN statement_descriptor_name,
    DROP COLUMN amount_to_capture,
    DROP COLUMN off_session,
    DROP COLUMN payment_confirm_source,
    DROP COLUMN merchant_order_reference_id,
    DROP COLUMN is_payment_processor_token_flow,
    DROP COLUMN charges;

-- Run below queries only when V1 is deprecated
ALTER TABLE payment_attempt DROP COLUMN attempt_id,
    DROP COLUMN amount,
    DROP COLUMN currency,
    DROP COLUMN save_to_locker,
    DROP COLUMN offer_amount,
    DROP COLUMN payment_method,
    DROP COLUMN connector_transaction_id,
    DROP COLUMN connector_transaction_data,
    DROP COLUMN processor_transaction_data,
    DROP COLUMN capture_method,
    DROP COLUMN capture_on,
    DROP COLUMN mandate_id,
    DROP COLUMN payment_method_type,
    DROP COLUMN business_sub_label,
    DROP COLUMN mandate_details,
    DROP COLUMN mandate_data,
    DROP COLUMN tax_amount,
    DROP COLUMN straight_through_algorithm,
    DROP COLUMN confirm,
    DROP COLUMN authentication_data,
    DROP COLUMN payment_method_billing_address_id,
    DROP COLUMN connector_mandate_detail,
    DROP COLUMN charge_id;

-- Run below queries only when V1 is deprecated
ALTER TABLE refund DROP COLUMN connector_refund_data,
    DROP COLUMN connector_transaction_data;

-- Run below queries only when V1 is deprecated
ALTER TABLE captures DROP COLUMN connector_capture_data;
