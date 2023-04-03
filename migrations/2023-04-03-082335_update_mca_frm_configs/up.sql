-- Your SQL goes here
-- ALTER TABLE merchant_connector_account
-- ADD COLUMN frm_configs TYPE json
-- USING frm_configs::json;
ALTER TABLE "merchant_connector_account" ADD COLUMN frm_configs jsonb;