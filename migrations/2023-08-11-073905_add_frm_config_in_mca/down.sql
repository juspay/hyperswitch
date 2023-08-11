ALTER TABLE "merchant_connector_account" DROP IF EXISTS COLUMN frm_config;
ALTER TABLE "merchant_connector_account" ADD COLUMN frm_configs jsonb[];