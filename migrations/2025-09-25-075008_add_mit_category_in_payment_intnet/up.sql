-- Add mit_category to payment_intent table
ALTER TABLE payment_intent
ADD COLUMN IF NOT EXISTS mit_category VARCHAR(64);
