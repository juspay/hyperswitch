-- This file should undo anything in `up.sql`
ALTER TABLE payment_link RENAME COLUMN seller_name TO custom_merchant_name;