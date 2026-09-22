ALTER TABLE invoice
ADD COLUMN IF NOT EXISTS billing_period_end TIMESTAMP NULL;

ALTER TABLE subscription
ADD COLUMN IF NOT EXISTS last_applied_billing_period_end TIMESTAMP NULL;
