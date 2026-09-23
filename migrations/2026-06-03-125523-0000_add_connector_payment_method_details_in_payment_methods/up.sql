-- Your SQL goes here
ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS connector_payment_method_details JSONB;