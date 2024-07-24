-- This file should undo anything in `up.sql`
ALTER TABLE customers ADD COLUMN IF EXISTS id;
ALTER TABLE customers DROP COLUMN IF NOT EXISTS id;

ALTER TABLE customers DROP CONSTRAINT customers_pkey;