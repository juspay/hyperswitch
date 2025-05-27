-- Your SQL goes here
ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS merchant_acquirer_ids text[];
