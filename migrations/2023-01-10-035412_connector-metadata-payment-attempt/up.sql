-- Your SQL goes here
ALTER TABLE payment_attempt ADD COLUMN connector_metadata JSONB DEFAULT NULL;