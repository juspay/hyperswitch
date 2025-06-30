-- Your SQL goes here
ALTER TABLE refund ADD COLUMN IF NOT EXISTS processor_merchant_id VARCHAR(64);
ALTER TABLE refund ADD COLUMN IF NOT EXISTS created_by VARCHAR(255);
UPDATE refund SET processor_merchant_id = merchant_id where processor_merchant_id IS NULL;