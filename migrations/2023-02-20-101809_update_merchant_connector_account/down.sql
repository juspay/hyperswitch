ALTER TABLE merchant_connector_account
DROP COLUMN IF EXISTS connector_label,
DROP COLUMN IF EXISTS business_country,
DROP COLUMN IF EXISTS business_label;

DROP INDEX IF EXISTS merchant_connector_account_merchant_id_connector_label_index;
CREATE UNIQUE INDEX merchant_connector_account_merchant_id_connector_name_index ON merchant_connector_account (merchant_id, connector_name);