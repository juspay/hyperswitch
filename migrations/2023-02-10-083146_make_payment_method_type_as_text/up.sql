-- Your SQL goes here
ALTER TABLE payment_methods
ALTER COLUMN payment_method_type TYPE VARCHAR(64);

ALTER TABLE payment_attempt
ADD COLUMN payment_method_type VARCHAR(64);

DROP TYPE IF EXISTS "PaymentMethodSubType";
