-- Your SQL goes here

ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS always_collect_billing_details_from_wallet_connector BOOLEAN DEFAULT FALSE;