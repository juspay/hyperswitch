-- Your SQL goes here
ALTER TABLE customers
ADD COLUMN IF NOT EXISTS default_payment_method_id VARCHAR(255);