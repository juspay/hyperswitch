-- This file should undo anything in `up.sql`
ALTER TABLE relay DROP COLUMN IF EXISTS processor_merchant_id;
ALTER TABLE relay DROP COLUMN IF EXISTS created_by;
