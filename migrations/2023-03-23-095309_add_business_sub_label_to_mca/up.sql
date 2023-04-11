ALTER TABLE merchant_connector_account
ADD COLUMN IF NOT EXISTS business_sub_label VARCHAR(64) DEFAULT 'default';
