-- This file should undo anything in `up.sql`
CREATE TYPE "PaymentMethodSubType" AS ENUM (
    'credit',
    'debit',
    'upi_intent',
    'upi_collect',
    'credit_card_installments',
    'pay_later_installments'
);

ALTER TABLE payment_attempt DROP COLUMN payment_method_type;

ALTER TABLE payment_methods
ALTER COLUMN payment_method_type TYPE "PaymentMethodSubType" USING payment_method_type::"PaymentMethodSubType";
