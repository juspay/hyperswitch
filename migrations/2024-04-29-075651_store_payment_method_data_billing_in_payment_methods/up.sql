-- Your SQL goes here
ALTER TABLE payment_methods
ADD COLUMN IF NOT EXISTS payment_method_billing_address BYTEA;
