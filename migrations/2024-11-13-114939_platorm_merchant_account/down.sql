-- This file should undo anything in `up.sql`
ALTER TABLE merchant_account DROP COLUMN IF EXISTS is_platform_account;

ALTER TABLE payment_intent DROP COLUMN IF EXISTS platform_merchant_id;
