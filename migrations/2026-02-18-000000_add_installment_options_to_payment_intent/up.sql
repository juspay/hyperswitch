-- Your SQL goes here
ALTER TABLE payment_intent
ADD COLUMN IF NOT EXISTS installment_options JSONB NULL;
