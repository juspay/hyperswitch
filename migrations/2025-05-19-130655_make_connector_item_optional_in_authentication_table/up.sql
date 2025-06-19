-- Your SQL goes here

ALTER TABLE authentication
ALTER COLUMN authentication_connector DROP NOT NULL,
ALTER COLUMN merchant_connector_id DROP NOT NULL;
