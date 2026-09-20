ALTER TABLE invoice
ADD COLUMN IF NOT EXISTS billing_period_end TIMESTAMP NULL;

ALTER TABLE subscription
ADD COLUMN IF NOT EXISTS last_applied_billing_period_end TIMESTAMP NULL;

CREATE INDEX CONCURRENTLY IF NOT EXISTS invoice_subscription_billing_period_end_index
    ON invoice (merchant_id, subscription_id, billing_period_end DESC)
    WHERE billing_period_end IS NOT NULL;
