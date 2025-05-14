-- Your SQL goes here
ALTER TABLE payment_intent
ADD COLUMN IF NOT EXISTS is_setup_mandate_flow boolean;