-- This file should undo anything in `up.sql`
ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS payment_method_id VARCHAR(64);

UPDATE payment_methods SET payment_method_id = id;