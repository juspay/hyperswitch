-- This file should undo anything in `up.sql`
ALTER TABLE file_metadata DROP COLUMN IF EXISTS merchant_connector_id;

ALTER TABLE refund DROP COLUMN IF EXISTS merchant_connector_id;

ALTER TABLE payout_attempt DROP COLUMN IF EXISTS merchant_connector_id;

ALTER TABLE dispute DROP COLUMN IF EXISTS merchant_connector_id;

ALTER TABLE mandate DROP COLUMN IF EXISTS merchant_connector_id;
