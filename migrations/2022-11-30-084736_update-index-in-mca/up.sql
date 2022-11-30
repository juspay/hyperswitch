DROP INDEX merchant_connector_account_connector_type_index;
DROP INDEX merchant_connector_account_merchant_id_index;
CREATE UNIQUE INDEX merchant_connector_account_merchant_id_connector_name_index ON merchant_connector_account (merchant_id, connector_name);