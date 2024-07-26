-- This file should undo anything in `up.sql`
ALTER TABLE customers ADD COLUMN customer_id VARCHAR(64) NOT NULL;