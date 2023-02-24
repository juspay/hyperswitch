-- Your SQL goes here
ALTER TABLE payment_attempt
ALTER COLUMN payment_method TYPE VARCHAR(50);

ALTER TABLE payment_methods
ALTER COLUMN payment_method TYPE VARCHAR(50);

DROP TYPE "PaymentMethodType";
