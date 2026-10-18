-- This file should undo anything in `up.sql`
ALTER TABLE payment_intent DROP COLUMN IF EXISTS is_account_funded_transaction;
ALTER TABLE payment_intent DROP COLUMN IF EXISTS recipient_details;
