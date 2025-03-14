-- Your SQL goes here
-- The below query will lock the user_roles table
-- Running this query is not necessary on higher environments
-- as the application will work fine without these queries being run
-- This query should be run after the new version of application is deployed
ALTER TABLE roles DROP CONSTRAINT roles_pkey;

-- Use the `role_id` column as primary key
-- These are already unique, not null column
-- So this query should not fail for not null or duplicate value reasons
ALTER TABLE roles
ADD PRIMARY KEY (role_id);
