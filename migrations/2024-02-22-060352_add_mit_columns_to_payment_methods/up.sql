-- Your SQL goes here

ALTER TABLE payment_methods
ADD COLUMN connector_mandate_details JSONB
DEFAULT NULL;

ALTER TABLE payment_methods
ADD COLUMN customer_acceptance JSONB
DEFAULT NULL;

ALTER TABLE payment_methods
ADD COLUMN status VARCHAR(64)
NOT NULL DEFAULT 'active';
