-- Your SQL goes here
ALTER TABLE merchant_connector_account
ADD COLUMN IF NOT EXISTS connector_webhook_details JSONB DEFAULT NULL;