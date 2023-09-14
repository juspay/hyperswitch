-- This file should undo anything in `up.sql`
ALTER TABLE address
ADD COLUMN id SERIAL;