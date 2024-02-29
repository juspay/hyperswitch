-- This file should undo anything in `up.sql`
ALTER TABLE customers DROP COLUMN IF EXISTS default_payment_method;
