-- Your SQL goes here
ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS payment_method_billing_address_id VARCHAR(64);
