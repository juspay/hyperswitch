
-- Renamed fields
-- payment_id is being renamed to merchant_reference_id
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS merchant_reference_id VARCHAR(64) NOT NULL;
-- billing_details is being renamed to billing_address
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS billing_address BYTEA DEFAULT NULL;
-- shipping_details is being renamed to shipping address
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS shipping_address BYTEA DEFAULT NULL;



-- New Fields
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS capture_method "CaptureMethod";

-- Run this query only when V1 is deprecated
ALTER TABLE payment_intent DROP CONSTRAINT IF EXISTS payment_intent_pkey;
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS id VARCHAR(64);
ALTER TABLE payment_intent ADD PRIMARY KEY (id);


-- Run below queries only when V1 is deprecated
-- Dropping columns that are renamed or not being included in V2
ALTER TABLE payment_intent DROP COLUMN payment_id;
ALTER TABLE payment_intent DROP COLUMN connector_id;
ALTER TABLE payment_intent DROP COLUMN shipping_address_id;
ALTER TABLE payment_intent DROP COLUMN billing_address_id;
ALTER TABLE payment_intent DROP COLUMN shipping_details;
ALTER TABLE payment_intent DROP COLUMN billing_details;
ALTER TABLE payment_intent DROP COLUMN statement_descriptor_suffix;
ALTER TABLE payment_intent DROP COLUMN business_country;
ALTER TABLE payment_intent DROP COLUMN business_label;
ALTER TABLE payment_intent DROP COLUMN incremental_authorization_allowed;
ALTER TABLE payment_intent DROP COLUMN fingerprint_id;
