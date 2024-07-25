-- Your SQL goes here
-- The below query will lock the mandate table
-- Running this query is not necessary on higher environments
-- as the application will work fine without these queries being run
-- This query is necessary for the application to not use id in update of mandate
-- This query should be run only after the new version of application is deployed
ALTER TABLE mandate DROP CONSTRAINT mandate_pkey;

-- Use the `mandate_id` column as primary key
ALTER TABLE mandate
ADD PRIMARY KEY (mandate_id);
