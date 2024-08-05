-- This file should undo anything in `up.sql`
ALTER TABLE merchant_account
ADD COLUMN IF NOT EXISTS id SERIAL;
