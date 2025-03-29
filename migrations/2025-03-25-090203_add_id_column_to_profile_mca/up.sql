-- Your SQL goes here
ALTER TABLE business_profile
ADD COLUMN id VARCHAR(64);

ALTER TABLE merchant_connector_account
ADD COLUMN id VARCHAR(64);