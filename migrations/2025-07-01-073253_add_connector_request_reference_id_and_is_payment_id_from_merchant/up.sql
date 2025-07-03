-- Your SQL goes here
ALTER TABLE payment_intent
ADD COLUMN IF NOT EXISTS is_payment_id_from_merchant boolean;

ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS connector_request_reference_id VARCHAR(255);