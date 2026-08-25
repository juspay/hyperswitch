-- This file should undo anything in `up.sql`
ALTER TABLE payment_methods DROP COLUMN IF EXISTS connector_payment_method_details;