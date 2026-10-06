-- Your SQL goes here
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS external_surcharge_strategy VARCHAR(64);
