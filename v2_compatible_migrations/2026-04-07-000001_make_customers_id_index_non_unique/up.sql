-- Replace the unique index on customers(id) with a non-unique index.
-- This allows the backfill migration to populate id from customer_id
-- even when there are duplicate customer_id values in the database.
-- Required because v1 used composite PK (merchant_id, customer_id),
-- so duplicate customer_id values exist across different merchants.
DROP INDEX IF EXISTS customers_id_index;
CREATE INDEX IF NOT EXISTS customers_id_index ON customers (id);
