-- This file should undo anything in `up.sql`
ALTER TABLE payment_attempt DROP COLUMN IF EXISTS client_version;
ALTER TABLE payment_attempt DROP COLUMN IF EXISTS client_source;