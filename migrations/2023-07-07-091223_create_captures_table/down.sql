
DROP INDEX authorized_attempt_id_index;
DROP INDEX connector_transaction_id_index;

DROP TABLE captures;
DROP TYPE "CaptureStatus";

ALTER TABLE payment_attempt
DROP COLUMN multiple_capture_count;