-- This file should undo anything in `up.sql`
ALTER TABLE payment_attempt DROP COLUMN IF EXISTS encrypted_payment_method_data;