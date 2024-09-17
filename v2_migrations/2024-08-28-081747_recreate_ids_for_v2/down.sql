-- This file contains queries to create the `id` column as a `SERIAL` column instead of `VARCHAR` column for tables that already have it.
-- This is to revert the `id` columns to the previous state.
ALTER TABLE merchant_account DROP id;

ALTER TABLE merchant_account
ADD COLUMN IF NOT EXISTS id SERIAL;

ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS id;

ALTER TABLE merchant_connector_account
ADD COLUMN IF NOT EXISTS id SERIAL;

ALTER TABLE customers DROP COLUMN IF EXISTS id;

ALTER TABLE customers
ADD COLUMN IF NOT EXISTS id SERIAL;

ALTER TABLE payment_intent DROP COLUMN IF EXISTS id;

ALTER TABLE payment_intent
ADD id SERIAL;

ALTER TABLE payment_attempt DROP COLUMN IF EXISTS id;

ALTER TABLE payment_attempt
ADD id SERIAL;
