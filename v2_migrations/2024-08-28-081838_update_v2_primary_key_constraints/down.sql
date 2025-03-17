-- Backfill for organization table
------------------------ Organization -----------------------
UPDATE ORGANIZATION
SET org_id = id
WHERE org_id IS NULL;

ALTER TABLE ORGANIZATION DROP CONSTRAINT organization_pkey_id;
ALTER TABLE ORGANIZATION DROP CONSTRAINT organization_organization_name_key;

-- Backfill
UPDATE ORGANIZATION
SET org_name = organization_name
WHERE org_name IS NULL AND organization_name IS NOT NULL;

------------------------ Merchant Account -----------------------
-- The new primary key for v2 merchant account will be `id`
ALTER TABLE merchant_account DROP CONSTRAINT merchant_account_pkey;
ALTER TABLE merchant_account ALTER COLUMN id DROP NOT NULL;

-- Backfill the id, a simple strategy will be to copy the values of id to merchant_id
UPDATE merchant_account
SET merchant_id = id
WHERE merchant_id IS NULL;

------------------------ Business Profile -----------------------
ALTER TABLE business_profile DROP CONSTRAINT business_profile_pkey;
ALTER TABLE business_profile ALTER COLUMN id DROP NOT NULL;

UPDATE business_profile
SET profile_id = id
WHERE profile_id IS NULL;

------------------------ Merchant Connector Account -----------------------
ALTER TABLE merchant_connector_account DROP CONSTRAINT merchant_connector_account_pkey;
ALTER TABLE merchant_connector_account ALTER COLUMN id DROP NOT NULL;

UPDATE merchant_connector_account
SET merchant_connector_id = id
WHERE merchant_connector_id IS NULL;

DROP INDEX IF EXISTS merchant_connector_account_profile_id_index;

------------------------ Customers -----------------------
-- Run this query only when V1 is deprecated
ALTER TABLE customers DROP CONSTRAINT customers_pkey;
ALTER TABLE customers ALTER COLUMN id DROP NOT NULL;

-- Backfill before making it primary key
UPDATE customers
SET customer_id = id
WHERE customer_id IS NULL;

------------------------ Payment Intent -----------------------
ALTER TABLE payment_intent DROP CONSTRAINT payment_intent_pkey;
ALTER TABLE payment_intent ALTER COLUMN id DROP NOT NULL;

UPDATE payment_intent
SET payment_id = id
WHERE payment_id IS NULL;

------------------------ Payment Attempt -----------------------
ALTER TABLE payment_attempt DROP CONSTRAINT payment_attempt_pkey;
ALTER TABLE payment_attempt ALTER COLUMN id DROP NOT NULL;

------------------------ Payment Methods -----------------------
ALTER TABLE payment_methods DROP CONSTRAINT payment_methods_pkey;
ALTER TABLE payment_methods ALTER COLUMN id DROP NOT NULL;
