-- Your SQL goes here
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS surcharge_strategy VARCHAR(64);
