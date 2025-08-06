-- This file contains queries to update the primary key constraints suitable to the v2 application.
-- This also has queries to update other constraints and indexes on tables where applicable.

------------------------ Organization -----------------------
-- Update null id and organization_name fields
UPDATE ORGANIZATION
SET id = org_id
WHERE id IS NULL;

UPDATE ORGANIZATION
SET organization_name = org_name
WHERE organization_name IS NULL AND org_name IS NOT NULL;

-- Alter queries for organization table
ALTER TABLE ORGANIZATION
ADD CONSTRAINT organization_pkey_id PRIMARY KEY (id);

ALTER TABLE ORGANIZATION
ADD CONSTRAINT organization_organization_name_key UNIQUE (organization_name);

------------------------ Merchant Account -----------------------
-- Backfill id column with merchant_id values
UPDATE merchant_account
SET id = merchant_id
WHERE id IS NULL;

-- Add primary key constraint
ALTER TABLE merchant_account
ADD PRIMARY KEY (id);

------------------------ Business Profile -----------------------
-- Backfill id column with profile_id values
UPDATE business_profile
SET id = profile_id
WHERE id IS NULL;

-- Add primary key constraint
ALTER TABLE business_profile
ADD PRIMARY KEY (id);

------------------------ Merchant Connector Account -----------------------
-- Backfill id column with merchant_connector_id values
UPDATE merchant_connector_account
SET id = merchant_connector_id
WHERE id IS NULL;

-- Add primary key constraint
ALTER TABLE merchant_connector_account
ADD PRIMARY KEY (id);

------------------------ Customers -----------------------
-- Backfill id column with customer_id values
UPDATE customers
SET id = customer_id
WHERE id IS NULL;

-- Add primary key constraint
ALTER TABLE customers
ADD PRIMARY KEY (id);

------------------------ Payment Intent -----------------------
-- Add primary key constraint
ALTER TABLE payment_intent
ADD PRIMARY KEY (id);

------------------------ Payment Attempt -----------------------
-- Add primary key constraint
ALTER TABLE payment_attempt
ADD PRIMARY KEY (id);

------------------------ Payment Methods -----------------------
-- Add primary key constraint
ALTER TABLE payment_methods 
ADD PRIMARY KEY (id);

------------------------ Refunds -----------------------
-- Add primary key constraint
ALTER TABLE refund
ADD PRIMARY KEY (id);