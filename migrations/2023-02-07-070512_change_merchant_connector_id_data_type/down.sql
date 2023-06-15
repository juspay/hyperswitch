CREATE SEQUENCE IF NOT EXISTS merchant_connector_id_seq OWNED BY merchant_connector_account.merchant_connector_id;

UPDATE merchant_connector_account
SET merchant_connector_id = id;

ALTER TABLE merchant_connector_account
ALTER COLUMN merchant_connector_id TYPE INTEGER USING (trim(merchant_connector_id)::integer);

ALTER TABLE merchant_connector_account
ALTER COLUMN merchant_connector_id SET DEFAULT nextval('merchant_connector_id_seq');
