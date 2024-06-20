-- Your SQL goes here
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS customer_details BYTEA DEFAULT NULL;
