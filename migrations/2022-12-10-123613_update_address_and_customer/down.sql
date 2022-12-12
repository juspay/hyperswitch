-- This file should undo anything in `up.sql`
ALTER TABLE address
DROP COLUMN customer_id,
DROP COLUMN merchant_id;

ALTER TABLE customers ADD COLUMN address JSON;