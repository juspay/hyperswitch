-- Dropping the constraint drops its backing index too.
ALTER TABLE customers DROP CONSTRAINT IF EXISTS customers_pkey;

ALTER TABLE customers ALTER COLUMN customer_id DROP NOT NULL;
