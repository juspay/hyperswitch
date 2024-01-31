-- This file should undo anything in `up.sql`
ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS status;
DROP TYPE IF EXISTS "ConnectorStatus";
