-- This file should undo anything in `up.sql`
DROP INDEX merchant_connector_account_merchant_id_connector_name_index;
CREATE INDEX merchant_connector_account_connector_type_index ON merchant_connector_account (connector_type);
CREATE INDEX merchant_connector_account_merchant_id_index ON merchant_connector_account (merchant_id);