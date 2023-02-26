ALTER TABLE merchant_connector_account
ALTER COLUMN merchant_connector_id TYPE VARCHAR(128) USING merchant_connector_id::varchar;


ALTER TABLE merchant_connector_account
ALTER COLUMN merchant_connector_id DROP DEFAULT;
