UPDATE merchant_connector_account set frm_configs = null ;

ALTER TABLE merchant_connector_account 
ALTER COLUMN frm_configs TYPE jsonb[]
USING ARRAY[frm_configs]::jsonb[];

UPDATE merchant_connector_account set frm_configs = null ;
