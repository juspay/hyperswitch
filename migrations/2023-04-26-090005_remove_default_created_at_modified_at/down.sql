-- Merchant Account
ALTER TABLE merchant_account
ALTER COLUMN modified_at SET DEFAULT now();

ALTER TABLE merchant_account
ALTER COLUMN created_at SET DEFAULT now();


-- Merchant Connector Account
ALTER TABLE merchant_connector_account
ALTER COLUMN modified_at SET DEFAULT now();

ALTER TABLE merchant_connector_account
ALTER COLUMN created_at SET DEFAULT now();

-- Customers
ALTER TABLE customers
ALTER COLUMN modified_at SET DEFAULT now();

ALTER TABLE customers
ALTER COLUMN created_at SET DEFAULT now();

-- Address
ALTER TABLE address
ALTER COLUMN modified_at SET DEFAULT now();

ALTER TABLE address
ALTER COLUMN created_at SET DEFAULT now();

-- Refunds
ALTER TABLE refund
ALTER COLUMN modified_at SET DEFAULT now();

ALTER TABLE refund
ALTER COLUMN created_at SET DEFAULT now();

-- Connector Response
ALTER TABLE connector_response
ALTER COLUMN modified_at SET DEFAULT now();

ALTER TABLE connector_response
ALTER COLUMN created_at SET DEFAULT now();

-- Payment methods
ALTER TABLE payment_methods
ALTER COLUMN created_at SET DEFAULT now();

-- Payment Intent
ALTER TABLE payment_intent
ALTER COLUMN modified_at SET DEFAULT now();

ALTER TABLE payment_intent
ALTER COLUMN created_at SET DEFAULT now();

--- Payment Attempt
ALTER TABLE payment_attempt
ALTER COLUMN modified_at SET DEFAULT now();

ALTER TABLE payment_attempt
ALTER COLUMN created_at SET DEFAULT now();
