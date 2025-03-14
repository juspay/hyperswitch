-- Your SQL goes here
-- The below query will lock the dispute table
-- Running this query is not necessary on higher environments
-- as the application will work fine without these queries being run
-- This query is necessary for the application to not use id in update of dispute
-- This query should be run only after the new version of application is deployed
ALTER TABLE dispute DROP CONSTRAINT dispute_pkey;

-- Use the `dispute_id` column as primary key
ALTER TABLE dispute
ADD PRIMARY KEY (dispute_id);
