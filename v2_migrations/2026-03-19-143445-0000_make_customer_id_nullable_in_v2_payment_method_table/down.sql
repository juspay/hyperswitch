-- This file should undo anything in `up.sql`
ALTER TABLE payment_methods
ALTER COLUMN customer_id SET NOT NULL;