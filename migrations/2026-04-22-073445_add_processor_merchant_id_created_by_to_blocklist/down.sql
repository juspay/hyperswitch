-- This file should undo anything in `up.sql`
ALTER TABLE blocklist DROP COLUMN IF EXISTS processor_merchant_id;
ALTER TABLE blocklist DROP COLUMN IF EXISTS created_by;
