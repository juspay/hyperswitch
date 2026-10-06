-- This file should undo anything in `up.sql`
ALTER TABLE incremental_authorization DROP COLUMN IF EXISTS processor_merchant_id;
