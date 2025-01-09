ALTER TABLE payment_attempt ALTER COLUMN connector_transaction_data TYPE VARCHAR(512);
ALTER TABLE refund ALTER COLUMN connector_refund_data TYPE VARCHAR(512);
ALTER TABLE refund ALTER COLUMN connector_transaction_data TYPE VARCHAR(512);
ALTER TABLE captures ALTER COLUMN connector_capture_data TYPE VARCHAR(512);