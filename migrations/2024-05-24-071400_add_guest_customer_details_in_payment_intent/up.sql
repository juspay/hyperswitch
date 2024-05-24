-- Your SQL goes here
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS guest_customer_data BYTEA DEFAULT NULL;