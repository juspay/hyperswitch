-- Your SQL goes here
ALTER TABLE payout_attempt ADD COLUMN IF NOT EXISTS payout_connector_metadata JSONB DEFAULT NULL;