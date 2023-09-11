-- Your SQL goes here
CREATE TYPE "PaymentSource" AS ENUM (
    'merchant_server',
    'postman',
    'dashboard',
    'sdk'
);

ALTER TABLE payment_intent
ADD COLUMN IF NOT EXISTS payment_confirm_source "PaymentSource";