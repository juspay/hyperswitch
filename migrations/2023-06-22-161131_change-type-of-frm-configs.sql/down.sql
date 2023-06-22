ALTER TABLE merchant_connector_account 
ALTER COLUMN frm_configs TYPE jsonb
USING frm_configs[1]::jsonb;