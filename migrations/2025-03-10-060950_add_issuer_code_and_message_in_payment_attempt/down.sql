ALTER TABLE payment_attempt
DROP COLUMN IF EXISTS issuer_error_code,
DROP COLUMN IF EXISTS issuer_error_message;

ALTER TABLE refund
DROP COLUMN IF EXISTS issuer_error_code,
DROP COLUMN IF EXISTS issuer_error_message;