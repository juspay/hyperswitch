-- Your SQL goes here
ALTER TABLE merchant_connector_account ADD COLUMN IF NOT EXISTS pm_auth_config JSONB DEFAULT NULL;
ALTER TYPE "ConnectorType" ADD VALUE 'payment_method_auth';