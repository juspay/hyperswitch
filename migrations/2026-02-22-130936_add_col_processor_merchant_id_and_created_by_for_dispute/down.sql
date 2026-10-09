-- This file should undo anything in `up.sql`
ALTER TABLE dispute DROP COLUMN IF EXISTS processor_merchant_id;
ALTER TABLE dispute DROP COLUMN IF EXISTS created_by;
