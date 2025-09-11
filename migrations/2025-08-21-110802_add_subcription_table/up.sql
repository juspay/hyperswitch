CREATE TABLE subscription (
  id SERIAL PRIMARY KEY,
  subscription_id VARCHAR(128) NOT NULL,
  status VARCHAR(128) NOT NULL,
  billing_processor VARCHAR(128),
  payment_method_id VARCHAR(128),
  mca_id VARCHAR(128),
  client_secret VARCHAR(128),
  connector_subscription_id VARCHAR(128),
  merchant_id VARCHAR(64) NOT NULL,
  customer_id VARCHAR(64) NOT NULL,
  metadata JSONB,
  created_at TIMESTAMP NOT NULL,
  modified_at TIMESTAMP NOT NULL
);

CREATE UNIQUE INDEX merchant_subscription_unique_index ON subscription (merchant_id, subscription_id);
