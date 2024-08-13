-- This adds back dropped columns in `up.sql`.
-- However, if the old columns were dropped, then we won't have data previously
-- stored in these columns.
ALTER TABLE business_profile
ADD COLUMN routing_algorithm JSON DEFAULT NULL,
    ADD COLUMN intent_fulfillment_time BIGINT DEFAULT NULL,
    ADD COLUMN frm_routing_algorithm JSONB DEFAULT NULL,
    ADD COLUMN payout_routing_algorithm JSONB DEFAULT NULL;

ALTER TABLE business_profile DROP COLUMN routing_algorithm_id,
    DROP COLUMN order_fulfillment_time,
    DROP COLUMN order_fulfillment_time_origin,
    DROP COLUMN frm_routing_algorithm_id,
    DROP COLUMN payout_routing_algorithm_id,
    DROP COLUMN default_fallback_routing;

DROP TYPE "OrderFulfillmentTimeOrigin";
