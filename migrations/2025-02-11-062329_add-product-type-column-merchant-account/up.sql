-- Your SQL goes here
ALTER TABLE merchant_account
ADD COLUMN IF NOT EXISTS product_type VARCHAR(64);