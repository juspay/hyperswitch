-- This file should undo anything in `up.sql`
ALTER TABLE payment_attempt
DROP COLUMN multiple_capture_count,
DROP COLUMN succeeded_capture_count;

ALTER TABLE captures
DROP COLUMN connector_transaction_id;
