-- Your SQL goes here
ALTER TABLE authentication ADD COLUMN IF NOT EXISTS acquirer_country_code VARCHAR(64);