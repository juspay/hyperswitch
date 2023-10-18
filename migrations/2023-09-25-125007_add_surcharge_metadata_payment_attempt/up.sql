-- Your SQL goes here
ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS surcharge_metadata JSONB DEFAULT NULL;