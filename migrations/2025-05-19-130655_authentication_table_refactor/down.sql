-- This file should undo anything in `up.sql`

ALTER TABLE authentication
ALTER COLUMN authentication_connector SET NOT NULL,
ALTER COLUMN merchant_connector_id SET NOT NULL;

ALTER TABLE authentication
ADD COLUMN IF NOT EXISTS authentication_client_secret VARCHAR(128) NULL;

ALTER TABLE authentication
DROP COLUMN IF EXISTS force_3ds_challenge;

ALTER TABLE authentication
DROP COLUMN IF EXISTS psd2_sca_exemption_type;

ALTER TABLE authentication
DROP COLUMN IF EXISTS return_url;

ALTER TABLE authentication
DROP COLUMN IF EXISTS amount;

ALTER TABLE authentication
DROP COLUMN IF EXISTS currency;
