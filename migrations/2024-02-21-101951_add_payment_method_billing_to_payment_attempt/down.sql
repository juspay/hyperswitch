-- This file should undo anything in `up.sql`
ALTER TABLE payment_attempt DROP COLUMN IF EXISTS payment_method_billing_address_id;
