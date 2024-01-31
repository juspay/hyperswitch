-- This file should undo anything in `up.sql`
ALTER TABLE payment_link DROP COLUMN IF EXISTS description;