-- Your SQL goes here
-- The below query will lock the user_roles table
-- Running this query is not necessary on higher environments
-- as the application will work fine without these queries being run
-- This query should be run after the new version of application is deployed
ALTER TABLE user_roles DROP CONSTRAINT user_roles_pkey;

-- Use the `user_id, merchant_id` columns as primary key
-- These are already unique, not null columns
-- So this query should not fail for not null or duplicate value reasons
ALTER TABLE user_roles
ADD PRIMARY KEY (user_id, merchant_id);
