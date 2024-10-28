ALTER TABLE payment_attempt
DROP COLUMN IF EXISTS connector_transaction_data;

ALTER TABLE refund
DROP COLUMN IF EXISTS connector_refund_data;

ALTER TABLE refund
DROP COLUMN IF EXISTS connector_transaction_data;

ALTER TABLE captures
DROP COLUMN IF EXISTS connector_capture_data;