-- Your SQL goes here
-- The below query will lock the refund table
-- Running this query is not necessary on higher environments
-- as the application will work fine without these queries being run
-- This query should be run after the new version of application is deployed
ALTER TABLE refund DROP CONSTRAINT refund_pkey;

-- Use the `merchant_id, refund_id` columns as primary key
-- These are already unique, not null columns
-- So this query should not fail for not null or duplicate value reasons
ALTER TABLE refund
ADD PRIMARY KEY (merchant_id, refund_id);
