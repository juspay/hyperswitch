-- This file should undo anything in `up.sql`
ALTER TABLE organization
DROP COLUMN IF EXISTS platform_merchant_id;