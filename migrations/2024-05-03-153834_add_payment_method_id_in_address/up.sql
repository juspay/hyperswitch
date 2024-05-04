-- Your SQL goes here
ALTER TABLE address
ADD COLUMN IF NOT EXISTS payment_method_id VARCHAR(64);
