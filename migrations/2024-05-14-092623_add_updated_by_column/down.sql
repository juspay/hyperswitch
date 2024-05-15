-- This file should undo anything in `up.sql`
ALTER TABLE payment_method DROP COLUMN IF EXISTS updated_by;

ALTER TABLE mandate DROP COLUMN IF EXISTS updated_by;

ALTER TABLE customers DROP COLUMN IF EXISTS updated_by;