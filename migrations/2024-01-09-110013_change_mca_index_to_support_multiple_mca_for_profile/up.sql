-- Your SQL goes here
DROP INDEX IF EXISTS merchant_connector_account_profile_id_connector_id_index;

CREATE UNIQUE INDEX IF NOT EXISTS merchant_connector_account_profile_id_connector_label ON merchant_connector_account(profile_id, connector_label);
