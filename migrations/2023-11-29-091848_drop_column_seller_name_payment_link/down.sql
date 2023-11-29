-- This file should undo anything in `up.sql`
ALTER TABLE payment_link ADD COLUMN seller_name character varying(64);
