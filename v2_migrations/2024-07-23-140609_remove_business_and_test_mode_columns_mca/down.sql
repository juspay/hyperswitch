ALTER TABLE merchant_connector_account ADD COLUMN IF NOT EXISTS business_country "CountryAlpha2";
ALTER TABLE merchant_connector_account ADD COLUMN IF NOT EXISTS business_label VARCHAR(255);
ALTER TABLE merchant_connector_account ADD COLUMN IF NOT EXISTS business_sub_label VARCHAR(64);
ALTER TABLE merchant_connector_account ADD COLUMN IF NOT EXISTS test_mode BOOLEAN;