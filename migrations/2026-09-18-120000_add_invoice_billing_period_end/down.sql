DROP INDEX CONCURRENTLY IF EXISTS invoice_subscription_billing_period_end_index;

ALTER TABLE subscription
DROP COLUMN IF EXISTS last_applied_billing_period_end;

ALTER TABLE invoice
DROP COLUMN IF EXISTS billing_period_end;
