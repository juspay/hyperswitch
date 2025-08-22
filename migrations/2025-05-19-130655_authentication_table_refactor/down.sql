-- This file should undo anything in `up.sql`

ALTER TABLE authentication
    ALTER COLUMN authentication_connector SET NOT NULL,
    ALTER COLUMN merchant_connector_id SET NOT NULL,
    DROP COLUMN IF EXISTS authentication_client_secret,
    DROP COLUMN IF EXISTS force_3ds_challenge,
    DROP COLUMN IF EXISTS psd2_sca_exemption_type,
    DROP COLUMN IF EXISTS return_url,
    DROP COLUMN IF EXISTS amount,
    DROP COLUMN IF EXISTS currency;
