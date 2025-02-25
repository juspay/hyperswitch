-- Your SQL goes here
ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS shipping_cost BIGINT;
ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS order_tax_amount BIGINT;
