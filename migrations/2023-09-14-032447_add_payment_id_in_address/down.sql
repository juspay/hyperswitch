-- This file should undo anything in `up.sql`
ALTER TABLE address DROP COLUMN payment_id;
ALTER TABLE customers DROP COLUMN address_id;