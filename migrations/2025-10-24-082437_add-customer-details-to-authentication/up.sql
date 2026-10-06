-- Your SQL goes here
ALTER TABLE authentication ADD COLUMN IF NOT EXISTS customer_details BYTEA DEFAULT NULL;