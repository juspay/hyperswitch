
DROP INDEX authorized_attempt_id_index;
DROP INDEX connector_transaction_id_index;

DROP TABLE captures;
DROP TYPE "CaptureStatus";

DELETE FROM pg_enum
WHERE enumlabel = 'partially_captured'
AND enumtypid = (
  SELECT oid FROM pg_type WHERE typname = 'IntentStatus'
);

ALTER TABLE payment_attempt
DROP COLUMN multiple_capture_count;