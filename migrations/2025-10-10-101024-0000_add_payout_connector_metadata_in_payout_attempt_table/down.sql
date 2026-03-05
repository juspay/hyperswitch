-- This file should undo anything in `up.sql`
ALTER TABLE payout_attempt DROP COLUMN IF EXISTS payout_connector_metadata;
