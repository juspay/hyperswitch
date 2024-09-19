-- This file should undo anything in `up.sql`
ALTER TABLE payment_intent DROP COLUMN IF EXISTS merchant_order_reference_id;