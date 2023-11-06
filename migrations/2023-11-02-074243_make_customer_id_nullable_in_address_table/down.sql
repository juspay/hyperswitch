-- This file should undo anything in `up.sql`
ALTER TABLE address ALTER COLUMN customer_id SET NOT NULL;