-- This file should undo anything in `up.sql`
ALTER TABLE captures ALTER COLUMN connector DROP NOT NULL;
ALTER TABLE captures RENAME COLUMN connector_capture_id TO connector_transaction_id;
ALTER TABLE captures DROP COLUMN IF EXISTS connector_response_reference_id;