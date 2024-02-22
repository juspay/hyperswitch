-- Your SQL goes here

CREATE TYPE "PaymentMethodStatus" AS ENUM (
    'active',
    'inactive',
    'processing'
);

ALTER TABLE payment_methods
ADD COLUMN connector_mit_details JSONB
DEFAULT NULL;

ALTER TABLE payment_methods
ADD COLUMN customer_acceptance JSONB
DEFAULT NULL;

ALTER TABLE payment_methods
ADD COLUMN status "PaymentMethodStatus"
NOT NULL DEFAULT 'active';
