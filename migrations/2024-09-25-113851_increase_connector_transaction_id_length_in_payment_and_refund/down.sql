ALTER TABLE payment_attempt
ALTER COLUMN connector_transaction_id TYPE VARCHAR(128);

ALTER TABLE refund
ALTER COLUMN connector_transaction_id TYPE VARCHAR(128);

ALTER TABLE refund
ALTER COLUMN connector_refund_id TYPE VARCHAR(128);