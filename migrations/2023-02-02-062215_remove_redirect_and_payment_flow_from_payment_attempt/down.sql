CREATE TYPE "PaymentFlow" AS ENUM (
    'vsc',
    'emi',
    'otp',
    'upi_intent',
    'upi_collect',
    'upi_scan_and_pay',
    'sdk'
);

ALTER TABLE payment_attempt
ADD COLUMN payment_flow "PaymentFlow";

ALTER TABLE payment_attempt
ADD COLUMN redirect BOOLEAN;
