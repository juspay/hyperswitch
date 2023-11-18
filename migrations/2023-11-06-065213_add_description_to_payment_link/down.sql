-- This file should undo anything in `up.sql`
ALTER table payment_link ADD COLUMN IF NOT EXISTS description VARCHAR (255);