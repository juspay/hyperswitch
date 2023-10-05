-- Your SQL goes here

CREATE TYPE "PaymentType" AS ENUM (
    'normal',
    'new_mandate',
    'setup_mandate',
    'recurring_mandate'
);

ALTER TABLE payment_intent
ADD COLUMN payment_type "PaymentType";
