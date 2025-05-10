-- This file should undo anything in `up.sql`
ALTER TABLE organization DROP COLUMN IF EXISTS organization_type;
ALTER TABLE organization DROP COLUMN IF EXISTS platform_merchant_id;

ALTER TABLE merchant_account DROP COLUMN IF EXISTS merchant_account_type;

