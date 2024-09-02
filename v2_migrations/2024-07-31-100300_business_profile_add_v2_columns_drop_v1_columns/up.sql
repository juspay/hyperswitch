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
ALTER TABLE business_profile DROP COLUMN routing_algorithm,
  DROP COLUMN intent_fulfillment_time,
  DROP COLUMN frm_routing_algorithm,
  DROP COLUMN payout_routing_algorithm;

-- This migration is to modify the id column in business_profile table to be a VARCHAR(64) and to set the id column as primary key
ALTER TABLE business_profile
ADD COLUMN id VARCHAR(64);

-- Backfill the id column with the profile_id to prevent null values
UPDATE business_profile
SET id = profile_id;

ALTER TABLE business_profile DROP CONSTRAINT business_profile_pkey;

ALTER TABLE business_profile
ADD PRIMARY KEY (id);

ALTER TABLE business_profile DROP COLUMN profile_id;
