-- Your SQL goes here
ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS always_request_overcapture BOOLEAN DEFAULT NULL;
ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS overcapture_status VARCHAR(32) DEFAULT NULL;
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS request_overcapture VARCHAR(32) DEFAULT NULL;