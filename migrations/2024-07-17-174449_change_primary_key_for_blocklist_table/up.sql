-- Your SQL goes here
-- The below query will lock the blocklist table
-- Use the `merchant_id, fingerprint_id` columns as primary key
-- These are already unique, not null columns
-- So this query should not fail for not null or duplicate value reasons
-- Running this query is not necessary on higher environments
-- as the application will work fine without these queries being run

ALTER TABLE blocklist DROP CONSTRAINT blocklist_pkey;

ALTER TABLE blocklist
ADD PRIMARY KEY (merchant_id, fingerprint_id);
