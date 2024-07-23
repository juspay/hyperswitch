-- Your SQL goes here
-- The below query will lock the payment_methods table
-- Running this query is not necessary on higher environments
-- as the application will work fine without these queries being run
-- This query should be run after the new version of application is deployed
ALTER TABLE payment_methods DROP CONSTRAINT payment_methods_pkey;

-- Use the `payment_method_id` column as primary key
-- This is already unique, not null column
-- So this query should not fail for not null or duplicate value reasons
ALTER TABLE payment_methods
ADD PRIMARY KEY (payment_method_id);
