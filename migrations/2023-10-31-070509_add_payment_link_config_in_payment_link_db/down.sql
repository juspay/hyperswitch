-- This file should undo anything in `up.sql`
ALTER TABLE payment_link DROP COLUMN IF EXISTS payment_link_config;
