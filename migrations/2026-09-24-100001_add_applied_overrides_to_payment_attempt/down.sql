-- Remove applied_overrides from payment_attempt table
ALTER TABLE payment_attempt DROP COLUMN IF EXISTS applied_overrides;
