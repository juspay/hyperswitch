-- This file should undo anything in `up.sql`

ALTER TABLE payment_link
ALTER COLUMN payment_link_config DROP NOT NULL;