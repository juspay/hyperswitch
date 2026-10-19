-- Your SQL goes here
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS sender_details BYTEA DEFAULT NULL;
