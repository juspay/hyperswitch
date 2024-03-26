-- This file should undo anything in `up.sql`
ALTER TABLE merchant_account
ALTER COLUMN organization_id DROP NOT NULL;
