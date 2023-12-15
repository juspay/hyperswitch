-- This file should undo anything in `up.sql`
ALTER TABLE payment_link ADD COLUMN custom_merchant_name VARCHAR(64);