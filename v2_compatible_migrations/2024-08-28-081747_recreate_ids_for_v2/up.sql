-- This file contains queries to re-create the `id` column as a `VARCHAR` column instead of `SERIAL` column for tables that already have it.
-- It must be ensured that the deployed version of the application does not include the `id` column in any of its queries.
-- Drop the id column as this will be used later as the primary key with a different type

------------------------ Merchant Connector Account -----------------------
-- This migration is to modify the id column in merchant_connector_account table to be a VARCHAR(64) and to set the id column as primary key

ALTER TABLE merchant_connector_account
ADD COLUMN IF NOT EXISTS id VARCHAR(64);

------------------------ Business Profile -----------------------
-- This migration is to modify the id column in business_profile table to be a VARCHAR(64) and to set the id column as primary key
ALTER TABLE business_profile
ADD COLUMN id VARCHAR(64);

------------------------ Customers -----------------------


ALTER TABLE customers
ADD COLUMN IF NOT EXISTS id VARCHAR(64);

------------------------ Payment Intent -----------------------


ALTER TABLE payment_intent
ADD COLUMN IF NOT EXISTS id VARCHAR(64);

------------------------ Payment Attempt -----------------------


ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS id VARCHAR(64);

------------------------ Payment Methods -----------------------


ALTER TABLE payment_methods 
ADD COLUMN IF NOT EXISTS id VARCHAR(64);

