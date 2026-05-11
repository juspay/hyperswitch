-- This file contains queries to create the `id` column as a `SERIAL` column instead of `VARCHAR` column for tables that already have it.
-- This is to revert the `id` columns to the previous state.

-- Note: customers `id` column is reverted via v1 migrations (migrations/2026-04-02-000001_add_id_column_to_customers)

ALTER TABLE payment_intent DROP COLUMN IF EXISTS id;



ALTER TABLE payment_attempt DROP COLUMN IF EXISTS id;


------------------------ Payment Methods -----------------------
ALTER TABLE payment_methods DROP COLUMN IF EXISTS id;
