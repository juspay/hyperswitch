ALTER TABLE "merchant_connector_account" ADD COLUMN frm_config jsonb[];
-- Do not run below migration in PROD as this only makes sandbox compatible to PROD version
ALTER TABLE merchant_connector_account 
ALTER COLUMN frm_configs TYPE jsonb
USING frm_configs[1]::jsonb;