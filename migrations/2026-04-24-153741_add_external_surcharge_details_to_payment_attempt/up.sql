-- Add external_surcharge_details to payment_attempt table
ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS external_surcharge_details JSONB;
