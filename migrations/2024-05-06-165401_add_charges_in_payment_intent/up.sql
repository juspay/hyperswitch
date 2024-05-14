ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS charges jsonb;

ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS charge_id VARCHAR(64);

ALTER TABLE refund ADD COLUMN IF NOT EXISTS charges jsonb;
