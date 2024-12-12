-- Your SQL goes here
ALTER TABLE refund ADD COLUMN IF NOT EXISTS split_refunds jsonb;