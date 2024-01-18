-- This file should undo anything in `up.sql`
ALTER TABLE payment_intent ALTER COLUMN request_incremental_authorization SET NOT NULL;