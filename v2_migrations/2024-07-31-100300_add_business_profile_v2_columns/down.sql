ALTER TABLE business_profile
    DROP COLUMN routing_algorithm_id,
    DROP COLUMN order_fulfillment_time,
    DROP COLUMN order_fulfillment_time_origin,
    DROP COLUMN frm_routing_algorithm_id,
    DROP COLUMN payout_routing_algorithm_id,
    DROP COLUMN default_fallback_routing;

DROP TYPE "OrderFulfillmentTimeOrigin";
