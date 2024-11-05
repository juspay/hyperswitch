-- Your SQL goes here
ALTER TABLE payment_intent
ADD COLUMN IF NOT EXISTS is_payment_processor_token_flow BOOLEAN;
