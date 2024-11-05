-- This file should undo anything in `up.sql`
ALTER TABLE payment_attempt DROP COLUMN IF EXISTS shipping_cost;
ALTER TABLE payment_attempt DROP COLUMN IF EXISTS order_tax_amount;