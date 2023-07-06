-- Your SQL goes here
ALTER TABLE payment_intent
ADD COLUMN allowed_payment_method_types JSON,
ADD COLUMN connector_metadata JSON,
ADD COLUMN feature_metadata JSON;
