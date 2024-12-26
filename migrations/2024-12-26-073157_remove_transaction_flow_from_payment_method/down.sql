-- This file should undo anything in `up.sql`
CREATE TYPE "PaymentDirection" AS ENUM (
  'payin',
  'payout'
);

ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS transaction_flow "PaymentDirection";