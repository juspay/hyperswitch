-- Your SQL goes here
ALTER TABLE captures ALTER COLUMN connector SET NOT NULL;
ALTER TABLE captures RENAME COLUMN connector_transaction_id TO connector_capture_id;
ALTER TABLE captures add COLUMN IF NOT EXISTS connector_response_reference_id VARCHAR(128);