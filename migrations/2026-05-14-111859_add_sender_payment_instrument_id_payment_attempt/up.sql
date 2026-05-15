-- Your SQL goes here
ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS sender_payment_instrument_id VARCHAR(255);