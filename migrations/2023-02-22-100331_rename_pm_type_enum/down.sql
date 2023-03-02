-- This file should undo anything in `up.sql`
ALTER TABLE payment_attempt
ADD COLUMN payment_issuer VARCHAR;

CREATE TYPE "PaymentMethodType" AS ENUM (
    'card',
    'bank_transfer',
    'netbanking',
    'upi',
    'open_banking',
    'consumer_finance',
    'wallet',
    'pay_later'
);

ALTER TABLE payment_attempt
ALTER COLUMN payment_method TYPE "PaymentMethodType" USING payment_method::"PaymentMethodType";

ALTER TABLE payment_methods
ALTER COLUMN payment_method TYPE "PaymentMethodType" USING payment_method::"PaymentMethodType";
