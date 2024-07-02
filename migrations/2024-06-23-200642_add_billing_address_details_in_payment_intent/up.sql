-- Your SQL goes here
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS billing_address_details BYTEA DEFAULT NULL;
