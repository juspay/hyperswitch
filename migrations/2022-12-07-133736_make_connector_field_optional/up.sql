ALTER TABLE payment_attempt ALTER COLUMN connector DROP NOT NULL;
ALTER TABLE connector_response ALTER COLUMN connector_name DROP NOT NULL;