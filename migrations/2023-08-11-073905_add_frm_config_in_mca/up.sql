ALTER TABLE "merchant_connector_account" ADD COLUMN frm_config jsonb[];
ALTER TABLE "merchant_connector_account" DROP COLUMN IF EXISTS frm_configs;