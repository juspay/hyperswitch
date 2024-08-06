-- This file should undo anything in `up.sql`
ALTER TABLE payment_methods DROP CONSTRAINT IF EXISTS payment_methods_pkey;
ALTER TABLE payment_methods DROP COLUMN IF EXISTS id;
ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS id SERIAL;