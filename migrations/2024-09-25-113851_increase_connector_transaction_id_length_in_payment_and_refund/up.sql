ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS connector_transaction_data VARCHAR(512);

ALTER TABLE refund
ADD COLUMN IF NOT EXISTS connector_refund_data VARCHAR(512);

ALTER TABLE refund
ADD COLUMN IF NOT EXISTS connector_transaction_data VARCHAR(512);

ALTER TABLE captures
ADD COLUMN IF NOT EXISTS connector_capture_data VARCHAR(512);