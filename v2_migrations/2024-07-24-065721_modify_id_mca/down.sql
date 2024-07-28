-- This file should undo anything in `up.sql`
ALTER TABLE merchant_connector_account DROP CONSTRAINT merchant_connector_account_pkey;

DROP INDEX IF EXISTS merchant_connector_account_id_index;

UPDATE merchant_connector_account SET merchant_connector_id = id;

ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS id;

ALTER TABLE merchant_connector_account ADD COLUMN IF NOT EXISTS id SERIAL;

ALTER TABLE merchant_connector_account ADD PRIMARY KEY (merchant_connector_id);
