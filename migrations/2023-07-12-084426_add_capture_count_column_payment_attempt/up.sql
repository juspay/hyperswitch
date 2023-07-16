-- Your SQL goes here
ALTER TABLE payment_attempt
ADD COLUMN multiple_capture_count SMALLINT, --number of captures available for this payment attempt in capture table
ADD COLUMN succeeded_capture_count SMALLINT; --number of succeeded captures available for this payment attempt in capture table

ALTER TABLE captures
ADD COLUMN connector_transaction_id VARCHAR(128),
ALTER COLUMN capture_sequence TYPE SMALLINT;

UPDATE payment_attempt
SET multiple_capture_count = 0, succeeded_capture_count = 0
WHERE payment_attempt.capture_method = 'manual_multiple';
