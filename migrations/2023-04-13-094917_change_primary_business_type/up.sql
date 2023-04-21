-- This change will allow older merchant accounts to be used with new changes
UPDATE merchant_account
SET primary_business_details = '[{"country": "US", "business": "default"}]';

-- Since this field is optional, default is not required
ALTER TABLE merchant_connector_account
ALTER COLUMN business_sub_label DROP DEFAULT;
