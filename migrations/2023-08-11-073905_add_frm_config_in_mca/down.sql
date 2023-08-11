ALTER TABLE "merchant_connector_account" DROP COLUMN frm_config;
ALTER TABLE "merchant_connector_account" ADD COLUMN frm_configs jsonb[];