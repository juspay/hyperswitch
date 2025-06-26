-- Your SQL goes here
ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS order_id VARCHAR(255);