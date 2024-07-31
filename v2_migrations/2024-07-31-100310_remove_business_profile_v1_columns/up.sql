-- Note: This query should not be run on higher environments as this leads to data loss.
-- The application will work fine even without these queries being run.
ALTER TABLE business_profile
    DROP COLUMN routing_algorithm,
    DROP COLUMN intent_fulfillment_time,
    DROP COLUMN frm_routing_algorithm,
    DROP COLUMN payout_routing_algorithm;
