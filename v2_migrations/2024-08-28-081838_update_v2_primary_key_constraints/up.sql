-- This file contains queries to update the primary key constraints suitable to the v2 application.
-- This also has queries to update other constraints and indexes on tables where applicable.
------------------------ Organization -----------------------
UPDATE ORGANIZATION
SET id = org_id
WHERE id IS NULL;

UPDATE ORGANIZATION
SET organization_name = org_name
WHERE organization_name IS NULL
    AND org_name IS NOT NULL;

-- Alter queries for organization table
ALTER TABLE ORGANIZATION
ADD CONSTRAINT organization_pkey_id PRIMARY KEY (id);

ALTER TABLE ORGANIZATION
ADD CONSTRAINT organization_organization_name_key UNIQUE (organization_name);

------------------------ Merchant Account -----------------------
-- The new primary key for v2 merchant account will be `id`
ALTER TABLE merchant_account DROP CONSTRAINT merchant_account_pkey;

-- In order to make id as primary key, it should be unique and not null
-- We need to backfill the id, a simple strategy will be to copy the values of merchant_id to id
-- Query to update the id column with values of merchant_id
-- Note: This query will lock the table, so it should be run when there is no traffic
UPDATE merchant_account
SET id = merchant_id
WHERE id IS NULL;

-- Note: This command might not run successfully for the existing table
-- This is because there will be some rows ( which are created via v1 application ) which will have id as empty
-- A backfill might be required to run this query
-- However if this is being run on a fresh database, this should succeed
ALTER TABLE merchant_account
ADD PRIMARY KEY (id);

------------------------ Business Profile -----------------------
-- This migration is to modify the id column in business_profile table to be a VARCHAR(64) and to set the id column as primary key
ALTER TABLE business_profile
ADD COLUMN id VARCHAR(64);

-- Backfill the id column with the profile_id to prevent null values
UPDATE business_profile
SET id = profile_id
WHERE id IS NULL;

ALTER TABLE business_profile DROP CONSTRAINT business_profile_pkey;

ALTER TABLE business_profile
ADD PRIMARY KEY (id);

------------------------ Merchant Connector Account -----------------------
-- Backfill the id column with the merchant_connector_id to prevent null values
UPDATE merchant_connector_account
SET id = merchant_connector_id
WHERE id IS NULL;

ALTER TABLE merchant_connector_account DROP CONSTRAINT merchant_connector_account_pkey;

ALTER TABLE merchant_connector_account
ADD PRIMARY KEY (id);

-- This migration is to make profile_id mandatory in mca table
ALTER TABLE merchant_connector_account
ALTER COLUMN profile_id
SET NOT NULL;

CREATE INDEX IF NOT EXISTS merchant_connector_account_profile_id_index ON merchant_connector_account (profile_id);

------------------------ Customers -----------------------
-- Run this query only when V1 is deprecated
ALTER TABLE customers DROP CONSTRAINT IF EXISTS customers_pkey;

-- Back filling before making it primary key
-- This will fail when making `id` as primary key, if the `customer_id` column has duplicate values.
-- Another option is to use a randomly generated ID instead.
UPDATE customers
SET id = customer_id
WHERE id IS NULL;

ALTER TABLE customers
ADD PRIMARY KEY (id);

------------------------ Payment Intent -----------------------
ALTER TABLE payment_intent DROP CONSTRAINT payment_intent_pkey;

ALTER TABLE payment_intent
ADD PRIMARY KEY (id);

------------------------ Payment Attempt -----------------------
ALTER TABLE payment_attempt DROP CONSTRAINT payment_attempt_pkey;

ALTER TABLE payment_attempt
ADD PRIMARY KEY (id);

-- This migration is to make fields mandatory in payment_intent table
ALTER TABLE payment_intent
ALTER COLUMN profile_id
SET NOT NULL,
    ALTER COLUMN currency
SET NOT NULL,
    ALTER COLUMN client_secret
SET NOT NULL,
    ALTER COLUMN session_expiry
SET NOT NULL,
    ALTER COLUMN active_attempt_id DROP NOT NULL;

------------------------ Payment Attempt -----------------------
ALTER TABLE payment_attempt DROP CONSTRAINT payment_attempt_pkey;

ALTER TABLE payment_attempt
ADD PRIMARY KEY (id);

-- This migration is to make fields mandatory in payment_attempt table
ALTER TABLE payment_attempt
ALTER COLUMN net_amount
SET NOT NULL,
    ALTER COLUMN authentication_type
SET NOT NULL,
    ALTER COLUMN payment_method_type_v2
SET NOT NULL,
    ALTER COLUMN payment_method_subtype
SET NOT NULL;

ALTER TABLE payment_intent
ALTER COLUMN session_expiry
SET NOT NULL;

-- This migration is to make fields optional in payment_intent table
ALTER TABLE payment_intent
ALTER COLUMN active_attempt_id DROP NOT NULL;

ALTER TABLE payment_intent
ALTER COLUMN active_attempt_id DROP DEFAULT;
