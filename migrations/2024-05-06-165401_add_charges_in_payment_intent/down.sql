ALTER TABLE payment_intent DROP COLUMN IF EXISTS charges;

ALTER TABLE payment_attempt DROP COLUMN IF EXISTS charge_id;

ALTER TABLE refund DROP COLUMN IF EXISTS charges;
