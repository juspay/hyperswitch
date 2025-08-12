-- Your SQL goes here
ALTER TABLE payment_methods
ADD COLUMN IF NOT EXISTS billing_connector_subscription_id VARCHAR(255);