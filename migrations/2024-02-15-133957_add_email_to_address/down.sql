-- This file should undo anything in `up.sql`
ALTER TABLE address DROP COLUMN IF EXISTS email;
