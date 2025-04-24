-- Your SQL goes here
ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS always_request_overcapture BOOLEAN;
ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS overcapture_status VARCHAR(32);
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS request_overcapture VARCHAR(32);