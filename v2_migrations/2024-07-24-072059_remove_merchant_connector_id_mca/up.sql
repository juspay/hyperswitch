-- Your SQL goes here
-- This migration is to remove the merchant_connector_id column from the merchant_connector_account table
ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS merchant_connector_id;
