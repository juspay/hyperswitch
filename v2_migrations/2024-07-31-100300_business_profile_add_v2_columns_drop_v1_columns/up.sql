CREATE TYPE "OrderFulfillmentTimeOrigin" AS ENUM ('create', 'confirm');

ALTER TABLE business_profile
    ADD COLUMN routing_algorithm_id VARCHAR(64) DEFAULT NULL,
    ADD COLUMN order_fulfillment_time BIGINT DEFAULT NULL,
    ADD COLUMN order_fulfillment_time_origin "OrderFulfillmentTimeOrigin" DEFAULT NULL,
    ADD COLUMN frm_routing_algorithm_id VARCHAR(64) DEFAULT NULL,
    ADD COLUMN payout_routing_algorithm_id VARCHAR(64) DEFAULT NULL,
    ADD COLUMN default_fallback_routing JSONB DEFAULT NULL;

-- Note: This query should not be run on higher environments as this leads to data loss.
-- The application will work fine even without these queries being run.
ALTER TABLE business_profile
    DROP COLUMN routing_algorithm,
    DROP COLUMN intent_fulfillment_time,
    DROP COLUMN frm_routing_algorithm,
    DROP COLUMN payout_routing_algorithm;
