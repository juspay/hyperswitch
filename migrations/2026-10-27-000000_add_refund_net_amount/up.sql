ALTER TABLE refund ADD COLUMN IF NOT EXISTS refund_surcharge_amount BIGINT;
ALTER TABLE refund ADD COLUMN IF NOT EXISTS net_refund_amount BIGINT;
