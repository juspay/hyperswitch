-- Your SQL goes here
-- The new primary key for v2 merchant account will be `id`
ALTER TABLE merchant_account DROP CONSTRAINT merchant_account_pkey;

-- In order to make id as primary key, it should be unique and not null
-- We need to backfill the id, a simple strategy will be to copy the values of merchant_id to id
-- Query to update the id column with values of merchant_id
-- Note: This query will lock the table, so it should be run when there is no traffic
UPDATE merchant_account
SET id = merchant_id;

-- Note: This command might not run successfully for the existing table
-- This is because there will be some rows ( which are created via v1 application ) which will have id as empty
-- A backfill might be required to run this query
-- However if this is being run on a fresh database, this should succeed
ALTER TABLE merchant_account
ADD PRIMARY KEY (id);
