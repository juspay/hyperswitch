-- Your SQL goes here
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS shipping_address_details BYTEA DEFAULT NULL;
