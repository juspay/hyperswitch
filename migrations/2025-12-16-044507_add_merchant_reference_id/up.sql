-- Your SQL goes here
ALTER TABLE payment_intent
ADD COLUMN IF NOT EXISTS merchant_reference_id VARCHAR(64);