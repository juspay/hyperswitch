-- This file should undo anything in `up.sql`
ALTER TABLE payment_method DROP COLUMN IF NOT EXISTS updated_by;

ALTER TABLE mandate DROP COLUMN IF NOT EXISTS updated_by;

ALTER TABLE customers DROP COLUMN IF NOT EXISTS updated_by;