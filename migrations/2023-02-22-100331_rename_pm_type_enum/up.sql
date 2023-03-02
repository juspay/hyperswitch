-- Your SQL goes here
ALTER TABLE payment_attempt
ALTER COLUMN payment_method TYPE VARCHAR;

ALTER TABLE payment_methods
ALTER COLUMN payment_method TYPE VARCHAR;

ALTER TABLE payment_methods
ALTER COLUMN payment_method_type TYPE VARCHAR;

ALTER TABLE payment_attempt DROP COLUMN payment_issuer;

ALTER TABLE payment_attempt
ADD COLUMN payment_method_data JSONB;

DROP TYPE "PaymentMethodType";
