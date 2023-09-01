-- Your SQL goes here
ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS card_details BYTEA DEFAULT NULL;