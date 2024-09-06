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
    DROP COLUMN default_fallback_routing;

DROP TYPE "OrderFulfillmentTimeOrigin";
