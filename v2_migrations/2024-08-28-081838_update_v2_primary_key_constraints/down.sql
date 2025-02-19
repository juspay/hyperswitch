-- Backfill for organization table
------------------------ Organization -----------------------
UPDATE ORGANIZATION
SET org_id = id
WHERE org_id IS NULL;

ALTER TABLE ORGANIZATION DROP CONSTRAINT organization_pkey_id;

ALTER TABLE ORGANIZATION DROP CONSTRAINT organization_organization_name_key;

-- back fill
UPDATE ORGANIZATION
SET org_name = organization_name
WHERE org_name IS NULL
    AND organization_name IS NOT NULL;

------------------------ Merchant Account -----------------------
-- The new primary key for v2 merchant account will be `id`
ALTER TABLE merchant_account DROP CONSTRAINT merchant_account_pkey;

-- In order to run this query, the merchant_id column should be unique and not null
-- We need to backfill the id, a simple strategy will be to copy the values of id to merchant_id
-- Query to update the merchant_id column with values of id
UPDATE merchant_account
SET merchant_id = id
WHERE merchant_id IS NULL;

-- Note: This command might not run successfully for the existing table
-- This is because there will be some rows ( which are created via v2 application ) which will have id as empty
-- A backfill might be required to run this query
-- However if this is being run on a fresh database, this should succeed
ALTER TABLE merchant_account
ADD PRIMARY KEY (merchant_id);

------------------------ Business Profile -----------------------
UPDATE business_profile
SET profile_id = id
WHERE profile_id IS NULL;

ALTER TABLE business_profile DROP COLUMN id;

ALTER TABLE business_profile
ADD PRIMARY KEY (profile_id);

------------------------ Merchant Connector Account -----------------------
ALTER TABLE merchant_connector_account DROP CONSTRAINT merchant_connector_account_pkey;

UPDATE merchant_connector_account
SET merchant_connector_id = id
WHERE merchant_connector_id IS NULL;

ALTER TABLE merchant_connector_account
ADD PRIMARY KEY (merchant_connector_id);

ALTER TABLE merchant_connector_account
ALTER COLUMN profile_id DROP NOT NULL;

DROP INDEX IF EXISTS merchant_connector_account_profile_id_index;

------------------------ Customers -----------------------
-- Run this query only when V1 is deprecated
ALTER TABLE customers DROP CONSTRAINT customers_pkey;

-- Back filling before making it primary key
UPDATE customers
SET customer_id = id
WHERE customer_id IS NULL;

ALTER TABLE customers
ADD PRIMARY KEY (merchant_id, customer_id);

------------------------ Payment Intent -----------------------
ALTER TABLE payment_intent DROP CONSTRAINT payment_intent_pkey;

UPDATE payment_intent
SET payment_id = id
WHERE payment_id IS NULL;

ALTER TABLE payment_intent
ADD PRIMARY KEY (payment_id, merchant_id);

ALTER TABLE payment_intent
ALTER COLUMN currency DROP NOT NULL,
    ALTER COLUMN client_secret DROP NOT NULL,
    ALTER COLUMN profile_id DROP NOT NULL;

ALTER TABLE payment_intent
ALTER COLUMN active_attempt_id
SET NOT NULL;

ALTER TABLE payment_intent
ALTER COLUMN session_expiry DROP NOT NULL;

ALTER TABLE payment_intent
ALTER COLUMN active_attempt_id
SET DEFAULT 'xxx';

------------------------ Payment Attempt -----------------------
ALTER TABLE payment_attempt DROP CONSTRAINT payment_attempt_pkey;

UPDATE payment_attempt
SET attempt_id = id
WHERE attempt_id IS NULL;

ALTER TABLE payment_attempt
ALTER COLUMN net_amount DROP NOT NULL;

ALTER TABLE payment_attempt
ADD PRIMARY KEY (attempt_id, merchant_id);
