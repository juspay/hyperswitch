ALTER TABLE payment_intent
DROP COLUMN IF EXISTS business_country,
DROP COLUMN IF EXISTS business_label;