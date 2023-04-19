-- This file should undo anything in `up.sql`
UPDATE merchant_account
SET primary_business_details = '{"country": ["US"], "business": ["default"]}';

ALTER TABLE merchant_connector_account
ALTER COLUMN business_sub_label
SET DEFAULT 'default';
