-- Your SQL goes here

CREATE TABLE subscription (
  id SERIAL PRIMARY KEY,
  subscription_id VARCHAR(128),
  biling_processor VARCHAR(128),
  payment_method_id VARCHAR(128),
  merchant_id VARCHAR(64) NOT NULL,
  customer_id VARCHAR(64) NOT NULL,
  metadata JSONB,
  created_at TIMESTAMP NOT NULL
);

CREATE UNIQUE INDEX merchant_customer_subscription_unique_index ON subscription (merchant_id, customer_id, subscription_id);
