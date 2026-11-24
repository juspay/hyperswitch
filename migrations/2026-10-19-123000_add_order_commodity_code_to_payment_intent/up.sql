-- Your SQL goes here

ALTER TABLE payment_intent
ADD COLUMN IF NOT EXISTS order_commodity_code VARCHAR;
