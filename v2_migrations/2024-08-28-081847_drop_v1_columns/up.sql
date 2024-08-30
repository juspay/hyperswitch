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
ALTER TABLE business_profile DROP COLUMN routing_algorithm,
    DROP COLUMN intent_fulfillment_time,
    DROP COLUMN frm_routing_algorithm,
    DROP COLUMN payout_routing_algorithm;

-- This migration is to remove the business_country, business_label, business_sub_label, test_mode, merchant_connector_id and frm_configs columns from the merchant_connector_account table
ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS business_country,
    DROP COLUMN IF EXISTS business_label,
    DROP COLUMN IF EXISTS business_sub_label,
    DROP COLUMN IF EXISTS test_mode,
    DROP COLUMN IF EXISTS merchant_connector_id,
    DROP COLUMN IF EXISTS frm_configs;

-- Run this query only when V1 is deprecated
ALTER TABLE customers DROP COLUMN customer_id,
    DROP COLUMN address_id;
