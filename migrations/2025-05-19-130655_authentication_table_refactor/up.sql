-- Your SQL goes here

ALTER TABLE authentication
ALTER COLUMN authentication_connector DROP NOT NULL,
ALTER COLUMN merchant_connector_id DROP NOT NULL;

ALTER TABLE authentication
ADD COLUMN IF NOT EXISTS authentication_client_secret VARCHAR(128) NULL;

ALTER TABLE authentication
ADD COLUMN IF NOT EXISTS force_3ds_challenge BOOLEAN NULL;

ALTER TABLE authentication
ADD COLUMN IF NOT EXISTS psd2_sca_exemption_type "ScaExemptionType" NULL;

ALTER TABLE authentication
ADD COLUMN IF NOT EXISTS return_url VARCHAR(2048) NULL;

ALTER TABLE authentication ADD COLUMN IF NOT EXISTS amount bigint;

ALTER TABLE authentication ADD COLUMN IF NOT EXISTS currency "Currency";