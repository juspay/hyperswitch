-- Your SQL goes here
ALTER TABLE business_profile
ADD COLUMN IF NOT EXISTS merchant_business_country "CountryAlpha2" DEFAULT NULL;