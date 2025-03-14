-- Your SQL goes here
ALTER TABLE business_profile
ADD COLUMN IF NOT EXISTS use_billing_as_payment_method_billing BOOLEAN DEFAULT TRUE;
