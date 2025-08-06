-- This file should undo anything in `up.sql`
ALTER TABLE payment_intent
DROP COLUMN IF EXISTS tax_status,
DROP COLUMN IF EXISTS discount_amount,
DROP COLUMN IF EXISTS shipping_amount_tax,
DROP COLUMN IF EXISTS duty_amount,
DROP COLUMN IF EXISTS order_date;

