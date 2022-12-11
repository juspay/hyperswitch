ALTER TABLE payment_attempt ALTER COLUMN connector SET NOT NULL;
ALTER TABLE connector_response ALTER COLUMN connector_name SET NOT NULL;