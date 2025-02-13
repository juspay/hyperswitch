ALTER TABLE payment_attempt
DROP COLUMN IF EXISTS processor_transaction_data;

ALTER TABLE refund DROP COLUMN IF EXISTS processor_refund_data;

ALTER TABLE refund DROP COLUMN IF EXISTS processor_transaction_data;

ALTER TABLE captures DROP COLUMN IF EXISTS processor_capture_data;