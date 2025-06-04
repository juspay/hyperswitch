-- This file contains queries to re-create the `id` column as a `VARCHAR` column instead of `SERIAL` column for tables that already have it.
-- It must be ensured that the deployed version of the application does not include the `id` column in any of its queries.
-- Drop the id column as this will be used later as the primary key with a different type
------------------------ Merchant Account -----------------------
ALTER TABLE merchant_account DROP COLUMN IF EXISTS id;

------------------------ Merchant Connector Account -----------------------
ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS id;


------------------------ Customers -----------------------
ALTER TABLE customers DROP COLUMN IF EXISTS id;



------------------------ Payment Intent -----------------------
ALTER TABLE payment_intent DROP COLUMN id;


------------------------ Payment Attempt -----------------------
ALTER TABLE payment_attempt DROP COLUMN id;


------------------------ Payment Methods -----------------------
ALTER TABLE payment_methods DROP COLUMN IF EXISTS id;

------------------------ Address -----------------------
ALTER TABLE address DROP COLUMN IF EXISTS id;

------------------------ Dispute -----------------------
ALTER TABLE dispute DROP COLUMN IF EXISTS id;

------------------------ Mandate -----------------------
ALTER TABLE mandate DROP COLUMN IF EXISTS id;

------------------------ Refund -----------------------
ALTER TABLE refund DROP COLUMN IF EXISTS id;

------------------------ BlockList -----------------------
ALTER TABLE blocklist DROP COLUMN IF EXISTS id;

------------------------ Roles -----------------------
ALTER TABLE roles DROP COLUMN IF EXISTS id;

------------------------ Users -----------------------
ALTER TABLE users DROP COLUMN IF EXISTS id;

