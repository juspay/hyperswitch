-- Your SQL goes here
-- This migration is to remove the business_country, business_label, business_sub_label, and test_mode columns from the merchant_connector_account table
ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS business_country;
ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS business_label;
ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS business_sub_label;
ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS test_mode;