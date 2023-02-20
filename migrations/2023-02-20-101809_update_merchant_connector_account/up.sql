ALTER TABLE merchant_connector_account
ADD COLUMN connector_label VARCHAR(255) NOT NULL,
ADD COLUMN connector_country VARCHAR(64) NOT NULL,
ADD COLUMN business_type VARCHAR(255) NOT NULL;

DROP INDEX merchant_connector_account_merchant_id_connector_name_index;

CREATE UNIQUE INDEX merchant_connector_account_merchant_id_connector_label_index ON merchant_connector_account (merchant_id, connector_label);