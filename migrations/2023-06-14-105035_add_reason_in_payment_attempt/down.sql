-- This file should undo anything in `up.sql`
ALTER TABLE payment_attempt
DROP COLUMN error_reason;
