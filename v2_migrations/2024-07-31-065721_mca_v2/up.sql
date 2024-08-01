-- Your SQL goes here
-- This migration is to remove the business_country, business_label, business_sub_label, and test_mode columns from the merchant_connector_account table
ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS business_country;
ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS business_label;
ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS business_sub_label;
ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS test_mode;

-- This migration is to modify the id column in merchant_connector_account table to be a VARCHAR(64) and to set the id column as primary key
ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS id;

ALTER TABLE merchant_connector_account ADD COLUMN IF NOT EXISTS id VARCHAR(64);

-- Backfill the id column with the merchant_connector_id to prevent null values
UPDATE merchant_connector_account SET id = merchant_connector_id;

CREATE UNIQUE INDEX merchant_connector_account_id_index ON merchant_connector_account (id);

ALTER TABLE merchant_connector_account DROP CONSTRAINT merchant_connector_account_pkey;

ALTER TABLE merchant_connector_account ADD PRIMARY KEY (id);

-- This migration is to remove the merchant_connector_id column from the merchant_connector_account table
ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS merchant_connector_id;

-- This migration is to remove the frm_configs column from the merchant_connector_account table
ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS frm_configs;