-- This file should undo anything in `up.sql`
ALTER TABLE payment_attempt DROP COLUMN IF EXISTS unified_code;
ALTER TABLE payment_attempt DROP COLUMN IF EXISTS unified_message;