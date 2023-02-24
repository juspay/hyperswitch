-- This file should undo anything in `up.sql`
CREATE TYPE "PaymentMethodType" AS ENUM (
    'card',
    'bank_transfer',
    'netbanking',
    'upi',
    'open_banking',
    'consumer_finance',
    'wallet',
    'payment_container',
    'bank_debit',
    'pay_later',
    'paypal'
);

ALTER TABLE payment_attempt
ALTER COLUMN payment_method TYPE "PaymentMethodType";

ALTER TABLE payment_methods
ALTER COLUMN payment_method TYPE "PaymentMethodType" NOT NULL;
