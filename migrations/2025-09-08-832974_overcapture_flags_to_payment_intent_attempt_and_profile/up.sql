ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS enable_overcapture BOOLEAN;

ALTER TABLE business_profile
ADD COLUMN always_enable_overcapture BOOLEAN;

ALTER TABLE payment_attempt
ADD COLUMN is_overcapture_enabled BOOLEAN;