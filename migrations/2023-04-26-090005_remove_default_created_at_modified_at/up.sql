-- Merchant Account
ALTER TABLE merchant_account
ALTER COLUMN modified_at DROP DEFAULT;

ALTER TABLE merchant_account
ALTER COLUMN created_at DROP DEFAULT;


-- Merchant Connector Account
ALTER TABLE merchant_connector_account
ALTER COLUMN modified_at DROP DEFAULT;

ALTER TABLE merchant_connector_account
ALTER COLUMN created_at DROP DEFAULT;

-- Customers
ALTER TABLE customers
ALTER COLUMN modified_at DROP DEFAULT;

ALTER TABLE customers
ALTER COLUMN created_at DROP DEFAULT;

-- Address
ALTER TABLE address
ALTER COLUMN modified_at DROP DEFAULT;

ALTER TABLE address
ALTER COLUMN created_at DROP DEFAULT;

-- Refunds
ALTER TABLE refund
ALTER COLUMN modified_at DROP DEFAULT;

ALTER TABLE refund
ALTER COLUMN created_at DROP DEFAULT;

-- Connector Response
ALTER TABLE connector_response
ALTER COLUMN modified_at DROP DEFAULT;

ALTER TABLE connector_response
ALTER COLUMN created_at DROP DEFAULT;

-- Payment methods
ALTER TABLE payment_methods
ALTER COLUMN created_at DROP DEFAULT;

-- Payment Intent
ALTER TABLE payment_intent
ALTER COLUMN modified_at DROP DEFAULT;

ALTER TABLE payment_intent
ALTER COLUMN created_at DROP DEFAULT;

--- Payment Attempt
ALTER TABLE payment_attempt
ALTER COLUMN modified_at DROP DEFAULT;

ALTER TABLE payment_attempt
ALTER COLUMN created_at DROP DEFAULT;
