-- Your SQL goes here
ALTER TABLE merchant_connector_account
ADD UNIQUE (profile_id, connector_label);

DROP INDEX IF EXISTS "merchant_connector_account_profile_id_connector_id_index";
