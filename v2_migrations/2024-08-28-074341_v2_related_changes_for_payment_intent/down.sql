
-- Revert dropping of columns
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS payment_id VARCHAR(64) NOT NULL;
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS connector_id VARCHAR(64);
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS shipping_address_id VARCHAR(64);
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS billing_address_id VARCHAR(64);
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS shipping_details VARCHAR(64);
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS billing_details VARCHAR(64);
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS statement_descriptor_suffix VARCHAR(255);
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS business_country "CountryAlpha2";
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS business_label VARCHAR(64);
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS incremental_authorization_allowed BOOLEAN;
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS fingerprint_id VARCHAR(64);

ALTER TABLE payment_intent DROP CONSTRAINT IF EXISTS payment_intent_pkey;
ALTER TABLE payment_intent DROP COLUMN IF EXISTS id;

ALTER TABLE payment_intent ADD PRIMARY KEY (merchant_id, payment_id);

-- Revert new fields added
ALTER TABLE payment_intent DROP COLUMN IF EXISTS capture_method;

-- Revert renaming of fields
ALTER TABLE payment_intent DROP COLUMN IF EXISTS merchant_reference_id;
ALTER TABLE payment_intent DROP COLUMN IF EXISTS billing_address;
ALTER TABLE payment_intent DROP COLUMN IF EXISTS shipping_address;
