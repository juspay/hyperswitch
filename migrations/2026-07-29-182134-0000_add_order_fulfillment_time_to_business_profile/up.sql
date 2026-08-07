ALTER TABLE business_profile
ADD COLUMN IF NOT EXISTS order_fulfillment_time BIGINT;

UPDATE business_profile
SET order_fulfillment_time = intent_fulfillment_time
WHERE order_fulfillment_time IS NULL
    AND intent_fulfillment_time IS NOT NULL;
