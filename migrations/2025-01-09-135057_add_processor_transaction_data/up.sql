ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS processor_transaction_data TEXT;

ALTER TABLE refund
ADD COLUMN IF NOT EXISTS processor_refund_data TEXT;

ALTER TABLE refund
ADD COLUMN IF NOT EXISTS processor_transaction_data TEXT;

ALTER TABLE captures
ADD COLUMN IF NOT EXISTS processor_capture_data TEXT;