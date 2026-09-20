ALTER TABLE subscription
DROP COLUMN IF EXISTS last_applied_billing_period_end;

ALTER TABLE invoice
DROP COLUMN IF EXISTS billing_period_end;
