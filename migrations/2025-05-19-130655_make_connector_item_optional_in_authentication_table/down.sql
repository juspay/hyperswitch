-- This file should undo anything in `up.sql`

ALTER TABLE authentication
ALTER COLUMN authentication_connector SET NOT NULL,
ALTER COLUMN merchant_connector_id SET NOT NULL;
