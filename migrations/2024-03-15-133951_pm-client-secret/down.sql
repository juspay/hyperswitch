-- This file should undo anything in `up.sql`
ALTER TABLE payment_methods DROP COLUMN IF EXISTS client_secret;
ALTER TABLE payment_methods ALTER COLUMN payment_method SET NOT NULL;