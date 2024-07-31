-- This adds back dropped columns in `up.sql`.
-- However, if the old columns were dropped, then we won't have data previously
-- stored in these columns.
ALTER TABLE business_profile
    ADD COLUMN routing_algorithm JSON DEFAULT NULL,
    ADD COLUMN intent_fulfillment_time BIGINT DEFAULT NULL,
    ADD COLUMN frm_routing_algorithm JSONB DEFAULT NULL,
    ADD COLUMN payout_routing_algorithm JSONB DEFAULT NULL;
