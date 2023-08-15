ALTER TABLE "merchant_connector_account" DROP COLUMN frm_config;
ALTER TABLE merchant_connector_account 
ALTER COLUMN frm_configs TYPE jsonb[]
USING ARRAY[frm_configs]::jsonb[];