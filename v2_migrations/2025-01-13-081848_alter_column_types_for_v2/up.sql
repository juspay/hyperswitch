-- Your SQL goes here
ALTER TABLE merchant_connector_account
    ALTER COLUMN payment_methods_enabled TYPE JSONB [ ];