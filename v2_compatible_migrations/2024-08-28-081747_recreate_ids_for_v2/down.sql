-- This file contains queries to create the `id` column as a `SERIAL` column instead of `VARCHAR` column for tables that already have it.
-- This is to revert the `id` columns to the previous state.

ALTER TABLE customers DROP COLUMN IF EXISTS id;



ALTER TABLE payment_intent DROP COLUMN IF EXISTS id;



ALTER TABLE payment_attempt DROP COLUMN IF EXISTS id;


------------------------ Payment Methods -----------------------
ALTER TABLE payment_methods DROP COLUMN IF EXISTS id;

