-- Your SQL goes here
ALTER TABLE refund
ADD COLUMN IF NOT EXISTS unified_code VARCHAR(255) DEFAULT NULL,
ADD COLUMN IF NOT EXISTS unified_message VARCHAR(1024) DEFAULT NULL;