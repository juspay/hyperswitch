ALTER TABLE merchant_account
ADD COLUMN IF NOT EXISTS primary_business_details JSON NOT NULL DEFAULT '{"country": ["US"], "business": ["default"]}';
