-- Your SQL goes here
ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS routing_strategy VARCHAR(64);