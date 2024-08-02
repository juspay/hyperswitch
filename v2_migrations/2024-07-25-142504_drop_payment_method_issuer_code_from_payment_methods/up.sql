-- Your SQL goes here
ALTER TABLE payment_methods DROP COLUMN IF EXISTS payment_method_issuer_code;

DROP TYPE IF EXISTS "PaymentMethodIssuerCode";