-- Add applied_overrides to payment_attempt table
ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS applied_overrides JSONB;
