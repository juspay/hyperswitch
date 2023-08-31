-- Your SQL goes here
ALTER TABLE payment_intent
ALTER COLUMN business_country DROP NOT NULL;

ALTER TABLE payment_intent
ALTER COLUMN business_label DROP NOT NULL;

ALTER TABLE merchant_connector_account
ALTER COLUMN business_country DROP NOT NULL;

ALTER TABLE merchant_connector_account
ALTER COLUMN business_label DROP NOT NULL;

ALTER TABLE merchant_connector_account
ALTER COLUMN connector_label DROP NOT NULL;

DROP INDEX IF EXISTS merchant_connector_account_merchant_id_connector_label_index;

CREATE UNIQUE INDEX IF NOT EXISTS merchant_connector_account_profile_id_connector_id_index ON merchant_connector_account(profile_id, connector_name);

CREATE UNIQUE INDEX IF NOT EXISTS business_profile_merchant_id_profile_name_index ON business_profile(merchant_id, profile_name);
