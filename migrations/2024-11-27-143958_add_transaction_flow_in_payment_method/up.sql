-- Your SQL goes here
ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS transaction_flow VARCHAR(255);