-- Your SQL goes here
ALTER TABLE merchant_connector_account ADD COLUMN IF NOT EXISTS additional_merchant_data BYTEA DEFAULT NULL;