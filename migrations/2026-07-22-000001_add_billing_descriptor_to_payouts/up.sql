ALTER TABLE payouts
ADD COLUMN IF NOT EXISTS billing_descriptor JSONB;
