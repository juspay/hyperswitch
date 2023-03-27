ALTER TABLE payment_intent
ADD COLUMN IF NOT EXISTS business_country VARCHAR(2),
ADD COLUMN IF NOT EXISTS business_label VARCHAR(64);
