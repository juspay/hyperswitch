-- This file should undo anything in `up.sql`

ALTER TABLE payment_methods
DROP COLUMN status;

ALTER TABLE payment_methods
DROP COLUMN customer_acceptance;

ALTER TABLE payment_methods
DROP COLUMN connector_mit_details;

DROP TYPE "PaymentMethodStatus";
