-- Your SQL goes here

ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS collect_shipping_details_from_wallet_connector BOOLEAN DEFAULT FALSE;