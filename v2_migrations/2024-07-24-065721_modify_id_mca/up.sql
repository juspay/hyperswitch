-- Your SQL goes here
ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS id;

ALTER TABLE merchant_connector_account ADD COLUMN IF NOT EXISTS id VARCHAR(64);

CREATE UNIQUE INDEX merchant_connector_account_id_index ON merchant_connector_account (id);

ALTER TABLE merchant_connector_account DROP CONSTRAINT merchant_connector_account_pkey;

ALTER TABLE merchant_connector_account ADD PRIMARY KEY (id);
