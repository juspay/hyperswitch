-- Your SQL goes here
ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS profile_acquirer_ids text[];
