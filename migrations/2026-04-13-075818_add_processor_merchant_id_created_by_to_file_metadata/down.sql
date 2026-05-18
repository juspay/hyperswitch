-- This file should undo anything in `up.sql`
ALTER TABLE file_metadata DROP COLUMN IF EXISTS processor_merchant_id;
ALTER TABLE file_metadata DROP COLUMN IF EXISTS created_by;