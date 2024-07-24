-- This file should undo anything in `up.sql`
ALTER TABLE customers ADD COLUMN address_id VARCHAR(64);