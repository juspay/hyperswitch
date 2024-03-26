ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS surcharge_metadata JSONB DEFAULT NULL;

ALTER TABLE payment_intent
DROP COLUMN surcharge_applicable;
