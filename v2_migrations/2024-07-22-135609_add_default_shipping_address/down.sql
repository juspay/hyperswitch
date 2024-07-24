-- This file should undo anything in `up.sql`
ALTER TABLE customers ADD COLUMN IF EXISTS default_shipping_address;