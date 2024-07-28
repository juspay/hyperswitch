-- Your SQL goes here
ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS business_country;
ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS business_label;
ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS business_sub_label;
ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS test_mode;