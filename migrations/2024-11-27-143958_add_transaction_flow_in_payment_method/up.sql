-- Your SQL goes here
CREATE TYPE "PaymentDirection" AS ENUM (
  'payin',
  'payout'
);

ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS transaction_flow "PaymentDirection";